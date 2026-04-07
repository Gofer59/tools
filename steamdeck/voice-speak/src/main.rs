// voice-speak — text-to-speech for highlighted text on Linux
//
// HOW IT WORKS
// ────────────
// 1. A background thread (rdev) watches every key event globally.
// 2. When the hotkey is pressed and no speech is playing:
//    a. Read the PRIMARY X11 selection (highlighted text) via xclip/wl-paste.
//    b. Fall back to the CLIPBOARD if PRIMARY is empty.
//    c. Spawn the Python TTS script as a subprocess to speak the text.
// 3. When the hotkey is pressed while speech IS playing:
//    Kill the TTS subprocess immediately (stop playback).
//
// THREADING MODEL
// ───────────────
//   main thread          ← orchestrates state machine + spawns/kills Python
//   rdev listener thread ← sends KeyEvent messages over a channel

use std::{
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Child, Command},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::{Context, Result};
use rdev::{listen, Event, EventType, Key};

// ─────────────────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// All tunable constants live here so a new reader can find them immediately.
struct Config {
    /// The hotkey that triggers TTS (speak) or stops it (if already speaking).
    /// Default: Right Alt.
    hotkey: Key,

    /// Path to the TTS wrapper script (activates venv + runs tts_speak.py).
    /// Resolved relative to the compiled binary so they can live together.
    tts_script: PathBuf,

    /// Piper voice model name (e.g. "en_US-lessac-medium").
    voice: String,

    /// Speech rate multiplier.  1.0 = normal speed.
    speed: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey: Key::ControlRight,
            tts_script: PathBuf::from(
                std::env::current_exe()
                    .unwrap_or_default()
                    .parent()
                    .unwrap_or(&PathBuf::from("."))
                    .join("tts_speak_wrapper.sh"),
            ),
            voice: "en_US-lessac-medium".into(),
            speed: 1.0,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Clipboard reading (Wayland only — uses wl-paste)
// ─────────────────────────────────────────────────────────────────────────────

/// Read the PRIMARY selection (highlighted text).  If empty, fall back to
/// the CLIPBOARD.  Returns the text or an empty string.
fn read_clipboard() -> Result<String> {
    let text = read_selection("primary")?;
    if !text.is_empty() {
        return Ok(text);
    }
    read_selection("clipboard")
}

/// Read a Wayland selection by name ("primary" or "clipboard") via wl-paste.
fn read_selection(selection: &str) -> Result<String> {
    let mut cmd = Command::new("wl-paste");
    if selection == "primary" {
        cmd.arg("--primary");
    }
    cmd.arg("--no-newline");
    let output = cmd.output().context(
        "Failed to run wl-paste. SteamOS: sudo pacman -S wl-clipboard",
    )?;

    // wl-paste exits with non-zero if the selection is empty.
    if !output.status.success() {
        return Ok(String::new());
    }

    let text = String::from_utf8(output.stdout)
        .context("Clipboard content was not valid UTF-8")?
        .trim()
        .to_owned();

    Ok(text)
}

// ─────────────────────────────────────────────────────────────────────────────
// TTS subprocess management
// ─────────────────────────────────────────────────────────────────────────────

/// Spawn the Python TTS script.  Returns the Child so the caller can kill it.
fn spawn_tts(text: &str, cfg: &Config) -> Result<Child> {
    println!(
        "[voice-speak] Speaking {} chars with voice '{}' (speed {})…",
        text.len(),
        cfg.voice,
        cfg.speed
    );

    // SAFETY: setsid() is async-signal-safe and has no preconditions.
    // We call it in pre_exec so the child gets its own process group.
    // This lets kill_tts() kill the entire tree (shell → python → paplay).
    let child = unsafe {
        Command::new(&cfg.tts_script)
            .arg(text)
            .arg(&cfg.voice)
            .arg(cfg.speed.to_string())
            .pre_exec(|| {
                libc::setsid();
                Ok(())
            })
            .spawn()
            .with_context(|| {
                format!(
                    "Failed to run TTS script at {:?}. Did you run install.sh?",
                    cfg.tts_script
                )
            })?
    };

    Ok(child)
}

/// Kill a running TTS subprocess and all its children (python, paplay).
///
/// Because we spawned the child with setsid(), its PID is also its process
/// group ID.  Sending SIGKILL to the negative PID kills the entire group.
fn kill_tts(child: &mut Child) {
    println!("[voice-speak] Stopping playback…");
    let pid = child.id() as i32;
    // Kill the entire process group (negative PID).
    unsafe { libc::kill(-pid, libc::SIGKILL); }
    let _ = child.wait();
}

// ─────────────────────────────────────────────────────────────────────────────
// Key event listener
// ─────────────────────────────────────────────────────────────────────────────

/// Runs the key-event listener loop.
///
/// `rdev::listen` is a blocking call that calls `callback` for every keyboard
/// and mouse event.  We run it in a dedicated thread and communicate with the
/// main thread via a channel.
fn spawn_key_listener(tx: std::sync::mpsc::Sender<EventType>) {
    std::thread::spawn(move || {
        if let Err(e) = listen(move |event: Event| {
            match &event.event_type {
                EventType::KeyPress(_) | EventType::KeyRelease(_) => {
                    let _ = tx.send(event.event_type);
                }
                _ => {}
            }
        }) {
            eprintln!("[voice-speak] rdev error: {:?}", e);
        }
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// State machine
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Idle,     // not speaking
    Speaking, // TTS subprocess is running
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let cfg = Config::default();

    println!("╔═══════════════════════════════════════╗");
    println!("║        voice-speak  ready              ║");
    println!("╠═══════════════════════════════════════╣");
    println!("║  Hotkey: {:?}{:>24}║", cfg.hotkey, "");
    println!("║  Voice:  {:<29}║", cfg.voice);
    println!("║  Speed:  {:<29}║", cfg.speed);
    println!("║  Clipboard: wl-paste (Wayland){:>8}║", "");
    println!("║  Press hotkey to speak selected text   ║");
    println!("║  Press again to stop playback          ║");
    println!("║  Ctrl-C to quit                        ║");
    println!("╚═══════════════════════════════════════╝");

    // Channel for key events from the listener thread → main thread.
    let (key_tx, key_rx) = std::sync::mpsc::channel::<EventType>();

    spawn_key_listener(key_tx);

    // Handle Ctrl-C gracefully.
    let running = Arc::new(AtomicBool::new(true));
    let ctrlc_flag = running.clone();
    ctrlc::set_handler(move || {
        ctrlc_flag.store(false, Ordering::SeqCst);
        println!("\n[voice-speak] Shutting down…");
        std::process::exit(0);
    })?;

    let mut state = State::Idle;
    let mut tts_child: Option<Child> = None;

    loop {
        let event = match key_rx.recv() {
            Ok(e) => e,
            Err(_) => break,
        };

        // We act on key PRESS only (not release) for a toggle-style hotkey.
        if let EventType::KeyPress(key) = event {
            if key != cfg.hotkey {
                continue;
            }

            match state {
                // ── Idle → Speaking ──────────────────────────────────────
                State::Idle => {
                    // Check if a previous child finished on its own.
                    // (It shouldn't be Some here, but be safe.)
                    tts_child = None;

                    let text = match read_clipboard() {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!("[voice-speak] Clipboard error: {e}");
                            continue;
                        }
                    };

                    if text.is_empty() {
                        println!("[voice-speak] Clipboard/selection is empty, nothing to speak.");
                        continue;
                    }

                    match spawn_tts(&text, &cfg) {
                        Ok(child) => {
                            tts_child = Some(child);
                            state = State::Speaking;
                        }
                        Err(e) => {
                            eprintln!("[voice-speak] TTS error: {e}");
                        }
                    }
                }

                // ── Speaking → Idle (stop) ───────────────────────────────
                State::Speaking => {
                    if let Some(ref mut child) = tts_child {
                        // Check if Python already exited on its own.
                        match child.try_wait() {
                            Ok(Some(_status)) => {
                                // Already finished — just reset state.
                                println!("[voice-speak] Playback already finished.");
                            }
                            _ => {
                                // Still running — kill it.
                                kill_tts(child);
                            }
                        }
                    }
                    tts_child = None;
                    state = State::Idle;
                    println!("[voice-speak] Ready. Press {:?} to speak again.", cfg.hotkey);
                }
            }
        }

        // Also detect when the TTS subprocess exits naturally.
        if state == State::Speaking {
            if let Some(ref mut child) = tts_child {
                if let Ok(Some(_)) = child.try_wait() {
                    tts_child = None;
                    state = State::Idle;
                    println!("[voice-speak] Playback finished. Ready.");
                }
            }
        }
    }

    Ok(())
}
