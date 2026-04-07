// voice-prompt — push-to-talk speech-to-text for any Linux terminal
//
// HOW IT WORKS
// ────────────
// 1. A background thread (rdev) watches every key event globally.
// 2. When the configured push-to-talk key is HELD, audio recording starts.
// 3. When the key is RELEASED, recording stops and samples are saved to a
//    temporary WAV file.
// 4. The Python transcription script is called with that WAV path.
// 5. The returned text is injected at the cursor via `xdotool type`.
//
// THREADING MODEL
// ───────────────
//   main thread          ← orchestrates state machine + calls Python
//   rdev listener thread ← sends KeyEvent messages over a channel
//   cpal stream thread   ← pushes audio samples over another channel

use std::{
    path::PathBuf,
    process::Command, // for xdotool and Python subprocess
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{Context, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat,
};
use rdev::{listen, Event, EventType, Key};
use tempfile::NamedTempFile;

// ─────────────────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// All tunable constants live here so a new reader can find them immediately.
struct Config {
    /// The modifier key that must already be held when `trigger_key` is pressed.
    /// Common choices: Key::Alt, Key::ControlLeft, Key::ShiftLeft
    ///
    /// Set to `None` to use a single key with no modifier (e.g. just F9).
    modifier_key: Option<Key>,

    /// The key that starts recording (when modifier is held) and stops it
    /// (on release).  Together with `modifier_key` this forms the chord.
    ///
    /// Example: modifier_key = Some(Key::Alt), trigger_key = Key::KeyS  →  Alt+S
    trigger_key: Key,

    /// Path to the Python transcription script.
    /// Adjust if you place the script elsewhere.
    python_script: PathBuf,

    /// Python interpreter to use.  `python3` works on most Linux systems.
    python_bin: String,

    /// Whisper model size.  Passed to the Python script.
    /// Options: "tiny", "base", "small", "medium", "large-v3"
    /// Larger = more accurate, slower.  "base" is a good default for CPU.
    whisper_model: String,

    /// Speech recognition language code.  Passed to the Python script.
    /// Valid values: "en" (English), "fr" (French).
    language: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // ── Push-to-talk chord: Alt + S ───────────────────────────────
            // Hold Alt, then press S to start recording.
            // Release S (Alt may stay held) to stop and transcribe.
            //
            // To change the chord, edit these two lines.  Examples:
            //   modifier_key: Some(Key::ControlLeft), trigger_key: Key::KeyF
            //   modifier_key: None,                  trigger_key: Key::F9
            modifier_key: Some(Key::MetaLeft),
            trigger_key:  Key::KeyS,
            python_script: PathBuf::from(
                // Resolve relative to the directory of the compiled binary,
                // so `voice-prompt` and `whisper_transcribe.py` can live together.
                std::env::current_exe()
                    .unwrap_or_default()
                    .parent()
                    .unwrap_or(&PathBuf::from("."))
                    .join("whisper_transcribe.py"),
            ),
            python_bin: "python3".into(),
            whisper_model: "small".into(),
            language: "en".into(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Audio helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Describes what we need to know about the audio device after opening it.
struct AudioSpec {
    sample_rate: u32,
    channels: u16,
}

/// Records audio into `samples` until `stop_flag` is set to `true`.
///
/// Uses `cpal`'s input stream.  Samples arrive as `f32` in the range [-1, 1].
/// We convert them to i16 (PCM 16-bit) before storing, because that is the
/// format Whisper and most audio tooling expects.
fn record_audio(stop_flag: Arc<AtomicBool>) -> Result<(Vec<i16>, AudioSpec)> {
    // Ask cpal for the default host (ALSA on Linux).
    let host = cpal::default_host();

    // The default input device is the one selected in the OS/PulseAudio mixer.
    let device = host
        .default_input_device()
        .context("No audio input device found. Is a microphone connected?")?;

    println!("[voice-prompt] Using device: {}", device.name().unwrap_or_default());

    // Ask the device what configuration it prefers.
    let supported_config = device
        .default_input_config()
        .context("Could not get default input configuration")?;

    let spec = AudioSpec {
        sample_rate: supported_config.sample_rate().0,
        channels: supported_config.channels(),
    };

    println!(
        "[voice-prompt] Recording at {} Hz, {} ch",
        spec.sample_rate, spec.channels
    );

    // A channel lets the cpal callback (which runs on its own thread) send
    // samples to us safely without needing a Mutex on a Vec.
    let (tx, rx) = std::sync::mpsc::channel::<Vec<i16>>();

    // Build the stream.  We match on the sample format so we can convert
    // correctly regardless of whether the device returns f32 or i16 natively.
    let stream = match supported_config.sample_format() {
        SampleFormat::F32 => {
            // Clone tx so the closure can own it while we keep rx.
            let tx = tx.clone();
            device.build_input_stream(
                &supported_config.into(),
                // This closure is called repeatedly by cpal with fresh buffers.
                move |data: &[f32], _info| {
                    // Convert f32 [-1.0, 1.0] → i16 [-32768, 32767]
                    let i16_samples: Vec<i16> = data
                        .iter()
                        .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                        .collect();
                    // Ignore send errors: the receiver may have dropped if we
                    // are shutting down.
                    let _ = tx.send(i16_samples);
                },
                |err| eprintln!("[voice-prompt] Audio stream error: {err}"),
                None, // no timeout
            )?
        }
        SampleFormat::I16 => {
            let tx = tx.clone();
            device.build_input_stream(
                &supported_config.into(),
                move |data: &[i16], _info| {
                    let _ = tx.send(data.to_vec());
                },
                |err| eprintln!("[voice-prompt] Audio stream error: {err}"),
                None,
            )?
        }
        other => {
            anyhow::bail!("Unsupported sample format: {:?}", other);
        }
    };

    // Start actually streaming audio.
    stream.play().context("Failed to start audio stream")?;

    // Collect all sample batches until the stop flag is raised.
    let mut all_samples: Vec<i16> = Vec::new();
    while !stop_flag.load(Ordering::Relaxed) {
        // Try to receive a batch; if nothing arrives in 50 ms we just loop.
        if let Ok(batch) = rx.recv_timeout(Duration::from_millis(50)) {
            all_samples.extend_from_slice(&batch);
        }
    }

    // Drain any samples that arrived between the flag flip and us exiting.
    while let Ok(batch) = rx.try_recv() {
        all_samples.extend_from_slice(&batch);
    }

    // Dropping `stream` stops the hardware capture.
    drop(stream);

    Ok((all_samples, spec))
}

// ─────────────────────────────────────────────────────────────────────────────
// WAV writing
// ─────────────────────────────────────────────────────────────────────────────

/// Writes `samples` (interleaved i16 PCM) into a temporary WAV file.
///
/// Returns the `NamedTempFile` so the caller keeps it alive until Python
/// finishes reading it (it is deleted automatically on Drop).
fn write_wav(samples: &[i16], spec: &AudioSpec) -> Result<NamedTempFile> {
    // NamedTempFile gives us a real filesystem path + auto-cleanup.
    let tmp = NamedTempFile::new().context("Could not create temporary WAV file")?;

    let wav_spec = hound::WavSpec {
        channels: spec.channels,
        sample_rate: spec.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    // BufWriter reduces the number of write() syscalls for large recordings.
    let buf_writer = std::io::BufWriter::new(
        tmp.reopen().context("Could not reopen temp file for writing")?,
    );

    let mut writer = hound::WavWriter::new(buf_writer, wav_spec)
        .context("Could not create WAV writer")?;

    for &sample in samples {
        writer.write_sample(sample).context("Failed to write WAV sample")?;
    }

    // hound writes the WAV header here.
    writer.finalize().context("Failed to finalize WAV file")?;

    Ok(tmp)
}

// ─────────────────────────────────────────────────────────────────────────────
// Transcription
// ─────────────────────────────────────────────────────────────────────────────

/// Calls the Python script as a subprocess and returns the transcript string.
///
/// The Python script receives three CLI arguments:
///   1. Path to the WAV file
///   2. Whisper model name
///   3. Language code ("en" or "fr")
fn transcribe(wav_path: &std::path::Path, cfg: &Config) -> Result<String> {
    println!("[voice-prompt] Transcribing with Whisper ({})…", cfg.whisper_model);

    let output = Command::new(&cfg.python_bin)
        .arg(&cfg.python_script)          // script path
        .arg(wav_path)                     // wav path    → sys.argv[1]
        .arg(&cfg.whisper_model)           // model name  → sys.argv[2]
        .arg(&cfg.language)               // language     → sys.argv[3]
        .output()                          // blocks until Python exits
        .with_context(|| {
            format!(
                "Failed to run Python. Is `{}` in your PATH?",
                cfg.python_bin
            )
        })?;

    if !output.status.success() {
        // Print Python's stderr so the user can diagnose missing packages etc.
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Python script failed:\n{}", stderr);
    }

    // Python prints only the transcript to stdout, nothing else.
    let transcript = String::from_utf8(output.stdout)
        .context("Python output was not valid UTF-8")?
        .trim()
        .to_owned();

    Ok(transcript)
}

// ─────────────────────────────────────────────────────────────────────────────
// Text injection
// ─────────────────────────────────────────────────────────────────────────────

/// Types `text` at the current cursor position using xdotool.
///
/// `xdotool type` synthesises X11 key events, so it works in any X11/Xwayland
/// application: terminal emulators, browsers, text editors, etc.
fn inject_text(text: &str) -> Result<()> {
    if text.is_empty() {
        println!("[voice-prompt] Transcript was empty, nothing to type.");
        return Ok(());
    }

    println!("[voice-prompt] Injecting: {:?}", text);

    // --clearmodifiers ensures held Shift/Ctrl keys don't garble the output.
    // --delay 0 types as fast as possible (default 12 ms between keypresses
    //   which is very slow for long transcripts).
    let status = Command::new("xdotool")
        .args(["type", "--clearmodifiers", "--delay", "0", "--", text])
        .status()
        .context("Failed to run xdotool. Install it with: sudo apt install xdotool")?;

    if !status.success() {
        anyhow::bail!("xdotool exited with non-zero status");
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Key event listener
// ─────────────────────────────────────────────────────────────────────────────

/// The four possible states our push-to-talk state machine can be in.
#[derive(Debug, Clone, Copy, PartialEq)]
enum PttState {
    Idle,       // key is up, not recording
    Recording,  // key is held, capturing audio
}

/// Runs the key-event listener loop.
///
/// `rdev::listen` is a blocking call that calls `callback` for every keyboard
/// and mouse event.  We run it in a dedicated thread and communicate with the
/// main thread via a channel.
fn spawn_key_listener(tx: std::sync::mpsc::Sender<EventType>) {
    std::thread::spawn(move || {
        // listen() never returns unless there is an error.
        if let Err(e) = listen(move |event: Event| {
            // We only care about key press / release; ignore mouse events.
            match &event.event_type {
                EventType::KeyPress(_) | EventType::KeyRelease(_) => {
                    // Best-effort send; if the receiver dropped we are shutting down.
                    let _ = tx.send(event.event_type);
                }
                _ => {}
            }
        }) {
            eprintln!("[voice-prompt] rdev error: {:?}", e);
        }
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

/// Parse the `-l` / `--language` flag from CLI arguments.
/// Returns the validated language code, defaulting to `"en"`.
fn print_usage() {
    eprintln!("Usage: voice-prompt [-l|--language <en|fr>] [-h|--help]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -l, --language <LANG>  Speech recognition language (en or fr, default: en)");
    eprintln!("  -h, --help             Show this help message and exit");
}

fn parse_language_arg() -> String {
    let args: Vec<String> = std::env::args().collect();
    let mut language: Option<String> = None;
    let mut i = 1; // skip argv[0] (binary name)
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            "-l" | "--language" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: {} requires a value (en or fr)", args[i]);
                    print_usage();
                    std::process::exit(1);
                }
                let lang = &args[i + 1];
                if lang != "en" && lang != "fr" {
                    eprintln!("Error: invalid language '{}'. Must be 'en' or 'fr'.", lang);
                    print_usage();
                    std::process::exit(1);
                }
                language = Some(lang.clone());
                i += 2; // skip flag + value
                continue;
            }
            other if other.starts_with('-') => {
                eprintln!("Warning: unrecognized option '{}'", other);
            }
            _ => {} // skip non-flag positional args
        }
        i += 1;
    }
    language.unwrap_or_else(|| "en".into())
}

fn main() -> Result<()> {
    let mut cfg = Config::default();
    cfg.language = parse_language_arg();

    // Print the chord description: "Alt+S" or just "F9" if no modifier.
    let chord_desc = match &cfg.modifier_key {
        Some(m) => format!("{:?}+{:?}", m, cfg.trigger_key),
        None    => format!("{:?}", cfg.trigger_key),
    };
    println!("╔═══════════════════════════════════════╗");
    println!("║        voice-prompt  ready            ║");
    println!("╠═══════════════════════════════════════╣");
    let lang_desc = match cfg.language.as_str() {
        "fr" => "fr (French)",
        _    => "en (English)",
    };
    println!("║  Chord: {:<31}║", chord_desc);
    println!("║  Language: {:<28}║", lang_desc);
    println!("║  Hold modifier, press key to record   ║");
    println!("║  Release key to transcribe + type     ║");
    println!("║  Ctrl-C to quit                       ║");
    println!("╚═══════════════════════════════════════╝");

    // Channel for key events from the listener thread → main thread.
    let (key_tx, key_rx) = std::sync::mpsc::channel::<EventType>();

    spawn_key_listener(key_tx);

    // Shared flag that tells the audio-recording thread to stop.
    // AtomicBool is lock-free; safe to share across threads via Arc.
    let stop_recording: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    // Handle Ctrl-C gracefully.
    let ctrlc_stop = stop_recording.clone();
    ctrlc::set_handler(move || {
        ctrlc_stop.store(true, Ordering::SeqCst);
        println!("\n[voice-prompt] Shutting down…");
        std::process::exit(0);
    })?;

    let mut state = PttState::Idle;

    // Tracks whether the modifier key (e.g. Alt) is currently held down.
    // We use a plain bool because only the main thread reads/writes it.
    let mut modifier_held: bool = false;

    // We spawn the recording thread inside the loop and join it here.
    // `Option<JoinHandle<…>>` lets us store it between iterations.
    let mut record_handle: Option<std::thread::JoinHandle<Result<(Vec<i16>, AudioSpec)>>> = None;

    // Process key events forever.
    loop {
        // Block until the next key event arrives.
        let event = match key_rx.recv() {
            Ok(e) => e,
            Err(_) => break, // listener thread died
        };

        match &event {
            // ── Track modifier key state (held / released) ────────────────
            // We need to know whether the modifier is down when the trigger
            // key arrives.  We track it with a simple bool.
            EventType::KeyPress(key) if cfg.modifier_key.as_ref() == Some(key) => {
                modifier_held = true;
            }
            EventType::KeyRelease(key) if cfg.modifier_key.as_ref() == Some(key) => {
                modifier_held = false;
            }

            // ── Transition: Idle → Recording ──────────────────────────────
            // Fires when the trigger key is pressed AND:
            //   • a modifier is configured and is currently held, OR
            //   • no modifier is configured (single-key mode).
            EventType::KeyPress(key)
                if *key == cfg.trigger_key
                    && state == PttState::Idle
                    && match &cfg.modifier_key {
                        Some(_) => modifier_held,  // chord mode: modifier must be down
                        None    => true,            // single-key mode: always fire
                    } =>
            {
                println!("[voice-prompt] ● Recording…");
                state = PttState::Recording;

                // Reset the stop flag before spawning the recording thread.
                stop_recording.store(false, Ordering::SeqCst);

                let flag = stop_recording.clone();
                record_handle = Some(std::thread::spawn(move || record_audio(flag)));
            }

            // ── Transition: Recording → Idle ──────────────────────────────
            // Stop as soon as the trigger key is released, regardless of
            // whether the modifier is still held.
            EventType::KeyRelease(key)
                if *key == cfg.trigger_key && state == PttState::Recording =>
            {
                println!("[voice-prompt] ■ Stopped. Processing…");
                state = PttState::Idle;

                // Signal the recording thread to stop.
                stop_recording.store(true, Ordering::SeqCst);

                // Wait for it to finish and collect the samples.
                if let Some(handle) = record_handle.take() {
                    match handle.join() {
                        Ok(Ok((samples, spec))) => {
                            // Pipeline: samples → WAV file → Python → xdotool
                            match write_wav(&samples, &spec) {
                                Ok(tmp_wav) => {
                                    match transcribe(tmp_wav.path(), &cfg) {
                                        Ok(text) => {
                                            if let Err(e) = inject_text(&text) {
                                                eprintln!("[voice-prompt] Inject error: {e}");
                                            }
                                        }
                                        Err(e) => eprintln!("[voice-prompt] Transcription error: {e}"),
                                    }
                                    // tmp_wav drops here → temp file deleted automatically
                                }
                                Err(e) => eprintln!("[voice-prompt] WAV write error: {e}"),
                            }
                        }
                        Ok(Err(e)) => eprintln!("[voice-prompt] Recording error: {e}"),
                        Err(_)     => eprintln!("[voice-prompt] Recording thread panicked"),
                    }
                }

                let chord_desc = match &cfg.modifier_key {
                    Some(m) => format!("{:?}+{:?}", m, cfg.trigger_key),
                    None    => format!("{:?}", cfg.trigger_key),
                };
                println!("[voice-prompt] Ready. Use {} to record again.", chord_desc);
            }

            // ── Ignore all other events ───────────────────────────────────
            _ => {}
        }
    }

    Ok(())
}
