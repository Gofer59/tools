// screen-ocr — hotkey-triggered screen region OCR for Linux
//
// HOW IT WORKS
// ────────────
// 1. A background thread (rdev) watches every key event globally.
// 2. Press F10 to draw a selection rectangle — the region geometry is saved.
// 3. Press F9 to re-capture the saved region instantly (no drawing needed).
// 4. The captured image is OCR'd via Tesseract (Python subprocess).
// 5. The text is copied to the clipboard.
// 6. The text is spoken aloud via Piper TTS (voice-speak infrastructure).
//
// DESIGNED FOR VISUAL NOVELS ON STEAMDECK
// ────────────────────────────────────────
// The dialogue text box in a visual novel is always in the same position.
// Select it once with F10, then press F9 for each new line of dialogue.
//
// THREADING MODEL
// ───────────────
//   main thread          ← orchestrates state machine + calls subprocesses
//   rdev listener thread ← sends KeyEvent messages over a channel

use std::{
    fs,
    io::Write,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::{Context, Result};
use rdev::{listen, Event, EventType, Key};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

// ─────────────────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// How the extracted text is delivered to the user.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum DeliveryMode {
    /// Copy to system clipboard (default — OCR text often needs editing).
    Clipboard,
    /// Type at cursor position via xdotool/ydotool.
    Type,
    /// Both: copy to clipboard AND type at cursor.
    Both,
}

/// All tunable constants live here so a new reader can find them immediately.
struct Config {
    /// Quick capture: re-use the stored region (default: F9).
    quick_capture_key: Key,

    /// Select new region interactively (default: F10).
    select_region_key: Key,

    /// Stop TTS playback (default: F11).
    stop_tts_key: Key,

    /// Path to the Python OCR wrapper script.
    python_script: PathBuf,

    /// How to deliver the extracted text.
    delivery_mode: DeliveryMode,

    /// Path to the voice-speak TTS wrapper script.
    tts_wrapper: PathBuf,

    /// Piper voice model name.
    tts_voice: String,

    /// Speech rate multiplier (1.0 = normal).
    tts_speed: String,

    /// Path to the saved region geometry JSON file.
    geometry_path: PathBuf,
}

/// Return `~/.local/share/screen-ocr`.
fn data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".local/share/screen-ocr")
}

impl Default for Config {
    fn default() -> Self {
        let bin_dir = std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or(&PathBuf::from("."))
            .to_path_buf();

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());

        Self {
            quick_capture_key: Key::F9,
            select_region_key: Key::F10,
            stop_tts_key: Key::F11,
            python_script: bin_dir.join("ocr_extract_wrapper.sh"),
            delivery_mode: DeliveryMode::Clipboard,
            tts_wrapper: PathBuf::from(&home).join(".local/bin/tts_speak_wrapper.sh"),
            tts_voice: "en_US-lessac-medium".into(),
            tts_speed: "1.0".into(),
            geometry_path: data_dir().join("last_region.json"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Display server detection
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum DisplayServer {
    X11,
    Wayland,
}

/// Detect whether we are running under X11 or Wayland.
fn detect_display_server() -> DisplayServer {
    if let Ok(session) = std::env::var("XDG_SESSION_TYPE") {
        if session.eq_ignore_ascii_case("wayland") {
            return DisplayServer::Wayland;
        }
    }
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        return DisplayServer::Wayland;
    }
    DisplayServer::X11
}

// ─────────────────────────────────────────────────────────────────────────────
// Region geometry
// ─────────────────────────────────────────────────────────────────────────────

/// A screen rectangle (x, y, width, height) in pixels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Region {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

/// Load the saved region from a JSON file.
fn load_region(path: &Path) -> Result<Region> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("No saved region at {:?}", path))?;
    let region: Region = serde_json::from_str(&contents)
        .context("Failed to parse saved region JSON")?;
    Ok(region)
}

/// Save a region to a JSON file (creates parent directories if needed).
fn save_region(path: &Path, region: &Region) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Cannot create directory {:?}", parent))?;
    }
    let json = serde_json::to_string_pretty(region)?;
    fs::write(path, json)?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Screen capture — interactive selection
// ─────────────────────────────────────────────────────────────────────────────

/// Let the user draw a selection rectangle and return its geometry.
///
/// - X11:     uses `slop` (outputs geometry in a custom format via --format)
/// - Wayland: uses `slurp` (outputs "X,Y WxH")
fn select_region(display: DisplayServer) -> Result<Region> {
    match display {
        DisplayServer::X11 => {
            // slop --format outputs space-separated: "W H X Y"
            let output = Command::new("slop")
                .args(["--format=%w %h %x %y"])
                .output()
                .context(
                    "Failed to run slop. Install with:\n  \
                     sudo apt install slop          # Debian/Ubuntu/Mint\n  \
                     sudo pacman -S slop            # Arch/SteamOS",
                )?;

            if !output.status.success() {
                anyhow::bail!(
                    "slop exited with non-zero status (user cancelled selection?)"
                );
            }

            let text = String::from_utf8(output.stdout)
                .context("slop output was not valid UTF-8")?
                .trim()
                .to_owned();

            // Parse "W H X Y"
            let parts: Vec<i32> = text
                .split_whitespace()
                .map(|s| s.parse::<i32>())
                .collect::<std::result::Result<Vec<_>, _>>()
                .with_context(|| format!("Failed to parse slop geometry: {:?}", text))?;

            if parts.len() != 4 {
                anyhow::bail!("Unexpected slop output (expected 4 values): {:?}", text);
            }

            Ok(Region {
                w: parts[0],
                h: parts[1],
                x: parts[2],
                y: parts[3],
            })
        }
        DisplayServer::Wayland => {
            let output = Command::new("slurp")
                .output()
                .context(
                    "Failed to run slurp. Install with:\n  \
                     sudo apt install slurp          # Debian/Ubuntu\n  \
                     sudo pacman -S slurp            # Arch/SteamOS",
                )?;

            if !output.status.success() {
                anyhow::bail!(
                    "slurp exited with non-zero status (user cancelled selection?)"
                );
            }

            let text = String::from_utf8(output.stdout)
                .context("slurp output was not valid UTF-8")?
                .trim()
                .to_owned();

            // Parse "X,Y WxH"
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.len() != 2 {
                anyhow::bail!("Unexpected slurp geometry: {:?}", text);
            }

            let xy: Vec<i32> = parts[0]
                .split(',')
                .map(|v| v.parse::<i32>())
                .collect::<std::result::Result<Vec<_>, _>>()
                .with_context(|| format!("Failed to parse slurp X,Y: {:?}", parts[0]))?;

            let wh: Vec<i32> = parts[1]
                .split('x')
                .map(|v| v.parse::<i32>())
                .collect::<std::result::Result<Vec<_>, _>>()
                .with_context(|| format!("Failed to parse slurp WxH: {:?}", parts[1]))?;

            if xy.len() != 2 || wh.len() != 2 {
                anyhow::bail!("Unexpected slurp geometry format: {:?}", text);
            }

            Ok(Region {
                x: xy[0],
                y: xy[1],
                w: wh[0],
                h: wh[1],
            })
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Screen capture — non-interactive (known geometry)
// ─────────────────────────────────────────────────────────────────────────────

/// Capture a known screen region to a file without any user interaction.
///
/// - X11:     `maim -g WxH+X+Y output.png`
/// - Wayland: `grim -g "X,Y WxH" output.png`
fn capture_geometry(display: DisplayServer, region: &Region, output_path: &Path) -> Result<()> {
    let path_str = output_path.to_string_lossy().to_string();

    match display {
        DisplayServer::X11 => {
            let geom = format!("{}x{}+{}+{}", region.w, region.h, region.x, region.y);
            let status = Command::new("maim")
                .args(["-g", &geom, &path_str])
                .status()
                .context(
                    "Failed to run maim. Install with:\n  \
                     sudo apt install maim          # Debian/Ubuntu/Mint\n  \
                     sudo pacman -S maim            # Arch/SteamOS",
                )?;

            if !status.success() {
                anyhow::bail!("maim exited with non-zero status");
            }
        }
        DisplayServer::Wayland => {
            let geom = format!("{},{} {}x{}", region.x, region.y, region.w, region.h);
            let status = Command::new("grim")
                .args(["-g", &geom, &path_str])
                .status()
                .context(
                    "Failed to run grim. Install with:\n  \
                     sudo apt install grim           # Debian/Ubuntu\n  \
                     sudo pacman -S grim             # Arch/SteamOS",
                )?;

            if !status.success() {
                anyhow::bail!("grim exited with non-zero status");
            }
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// OCR extraction
// ─────────────────────────────────────────────────────────────────────────────

/// Calls the Python OCR script and returns the extracted text.
fn ocr_extract(image_path: &Path, cfg: &Config) -> Result<String> {
    println!("[screen-ocr] Running OCR…");

    let output = Command::new(&cfg.python_script)
        .arg(image_path)
        .output()
        .with_context(|| {
            format!(
                "Failed to run OCR script at {:?}. Did you run install.sh?",
                cfg.python_script
            )
        })?;

    // Print Python's stderr (diagnostic messages) to our stderr.
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        eprint!("{}", stderr);
    }

    if !output.status.success() {
        anyhow::bail!("OCR script failed:\n{}", stderr);
    }

    let text = String::from_utf8(output.stdout)
        .context("OCR output was not valid UTF-8")?
        .trim()
        .to_owned();

    Ok(text)
}

// ─────────────────────────────────────────────────────────────────────────────
// Text delivery
// ─────────────────────────────────────────────────────────────────────────────

/// Copy text to the system clipboard.
fn copy_to_clipboard(text: &str, display: DisplayServer) -> Result<()> {
    match display {
        DisplayServer::X11 => {
            let mut child = Command::new("xclip")
                .args(["-selection", "clipboard"])
                .stdin(Stdio::piped())
                .spawn()
                .context(
                    "Failed to run xclip. Install with: sudo apt install xclip",
                )?;

            if let Some(ref mut stdin) = child.stdin {
                stdin
                    .write_all(text.as_bytes())
                    .context("Failed to write to xclip stdin")?;
            }

            let status = child.wait().context("Failed to wait for xclip")?;
            if !status.success() {
                anyhow::bail!("xclip exited with non-zero status");
            }
        }
        DisplayServer::Wayland => {
            let mut child = Command::new("wl-copy")
                .stdin(Stdio::piped())
                .spawn()
                .context(
                    "Failed to run wl-copy. Install with:\n  \
                     sudo apt install wl-clipboard   # Debian/Ubuntu\n  \
                     sudo pacman -S wl-clipboard     # Arch/SteamOS",
                )?;

            if let Some(ref mut stdin) = child.stdin {
                stdin
                    .write_all(text.as_bytes())
                    .context("Failed to write to wl-copy stdin")?;
            }

            let status = child.wait().context("Failed to wait for wl-copy")?;
            if !status.success() {
                anyhow::bail!("wl-copy exited with non-zero status");
            }
        }
    }

    Ok(())
}

/// Type text at the current cursor position.
fn type_text(text: &str, display: DisplayServer) -> Result<()> {
    match display {
        DisplayServer::X11 => {
            let status = Command::new("xdotool")
                .args(["type", "--clearmodifiers", "--delay", "0", "--", text])
                .status()
                .context(
                    "Failed to run xdotool. Install with: sudo apt install xdotool",
                )?;

            if !status.success() {
                anyhow::bail!("xdotool exited with non-zero status");
            }
        }
        DisplayServer::Wayland => {
            let status = Command::new("ydotool")
                .args(["type", "--", text])
                .status()
                .context(
                    "Failed to run ydotool. Install with:\n  \
                     sudo pacman -S ydotool          # Arch/SteamOS\n  \
                     Note: ydotoold daemon must be running.",
                )?;

            if !status.success() {
                anyhow::bail!("ydotool exited with non-zero status");
            }
        }
    }

    Ok(())
}

/// Deliver the extracted text according to the configured mode.
fn deliver_text(text: &str, display: DisplayServer, mode: DeliveryMode) -> Result<()> {
    if text.is_empty() {
        println!("[screen-ocr] No text extracted, nothing to deliver.");
        return Ok(());
    }

    match mode {
        DeliveryMode::Clipboard => {
            copy_to_clipboard(text, display)?;
            println!("[screen-ocr] Copied {} chars to clipboard.", text.len());
        }
        DeliveryMode::Type => {
            type_text(text, display)?;
            println!("[screen-ocr] Typed {} chars at cursor.", text.len());
        }
        DeliveryMode::Both => {
            copy_to_clipboard(text, display)?;
            type_text(text, display)?;
            println!("[screen-ocr] Copied + typed {} chars.", text.len());
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// TTS (text-to-speech via voice-speak / Piper)
// ─────────────────────────────────────────────────────────────────────────────

/// Spawn the TTS wrapper as a detached subprocess.
///
/// The child gets its own process group via `setsid()` so that `kill_tts()`
/// can kill the entire tree (shell → python → paplay) at once.
fn spawn_tts(text: &str, cfg: &Config) -> Result<Child> {
    println!("[screen-ocr] Speaking {} chars…", text.len());

    // SAFETY: setsid() is async-signal-safe and has no preconditions.
    let child = unsafe {
        Command::new(&cfg.tts_wrapper)
            .arg(text)
            .arg(&cfg.tts_voice)
            .arg(&cfg.tts_speed)
            .pre_exec(|| {
                libc::setsid();
                Ok(())
            })
            .spawn()
            .with_context(|| {
                format!(
                    "Failed to run TTS wrapper at {:?}. Is voice-speak installed?",
                    cfg.tts_wrapper
                )
            })?
    };

    Ok(child)
}

/// Kill a running TTS subprocess and all its children (python, paplay).
fn kill_tts(child: &mut Child) {
    println!("[screen-ocr] Stopping previous TTS…");
    let pid = child.id() as i32;
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
    }
    let _ = child.wait();
}

// ─────────────────────────────────────────────────────────────────────────────
// Pipeline: capture → OCR → clipboard → TTS
// ─────────────────────────────────────────────────────────────────────────────

/// Run the full OCR pipeline on a known region.
///
/// 1. Capture the region to a temp PNG.
/// 2. Run OCR on the image.
/// 3. Copy the text to the clipboard.
/// 4. Kill any previous TTS and speak the new text.
fn run_pipeline(
    display: DisplayServer,
    region: &Region,
    cfg: &Config,
    tts_child: &mut Option<Child>,
) -> Result<()> {
    // 1. Capture
    let tmp = NamedTempFile::with_suffix(".png")
        .context("Could not create temporary PNG file")?;
    capture_geometry(display, region, tmp.path())?;

    // 2. OCR
    let text = ocr_extract(tmp.path(), cfg)?;
    // tmp drops here → temp file deleted automatically

    // 3. Clipboard
    deliver_text(&text, display, cfg.delivery_mode)?;

    // 4. TTS (non-blocking)
    if !text.is_empty() {
        // Kill previous TTS if still running
        if let Some(ref mut child) = tts_child {
            match child.try_wait() {
                Ok(Some(_)) => {} // already finished
                _ => kill_tts(child),
            }
        }
        match spawn_tts(&text, cfg) {
            Ok(child) => *tts_child = Some(child),
            Err(e) => {
                eprintln!("[screen-ocr] TTS error (continuing without speech): {e}");
                *tts_child = None;
            }
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Key event listener
// ─────────────────────────────────────────────────────────────────────────────

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
            eprintln!("[screen-ocr] rdev error: {:?}", e);
        }
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let cfg = Config::default();
    let display = detect_display_server();

    let capture_tool = match display {
        DisplayServer::X11 => "slop + maim -g",
        DisplayServer::Wayland => "slurp + grim -g",
    };

    let clipboard_tool = match display {
        DisplayServer::X11 => "xclip",
        DisplayServer::Wayland => "wl-copy",
    };

    let tts_available = cfg.tts_wrapper.exists();

    // Check for saved region
    let has_region = cfg.geometry_path.exists();

    println!("╔════════════════════════════════════════════╗");
    println!("║         screen-ocr  ready                   ║");
    println!("╠════════════════════════════════════════════╣");
    println!("║  F9:       Quick capture (re-use region)    ║");
    println!("║  F10:      Select new region                ║");
    println!("║  F11:      Stop TTS playback                ║");
    println!("║  Display:  {:?}{:<30}║", display, "");
    println!("║  Capture:  {:<30}║", capture_tool);
    println!("║  Clipboard:{:<30}║", clipboard_tool);
    println!("║  TTS:      {:<30}║",
        if tts_available { "Piper (voice-speak)" } else { "not available" });
    println!("║  Region:   {:<30}║",
        if has_region { "loaded from disk" } else { "none (use F10 first)" });
    println!("║                                             ║");
    println!("║  F10 → draw region → OCR → clipboard → TTS  ║");
    println!("║  F9  → instant re-capture → OCR → TTS       ║");
    println!("║  Ctrl-C to quit                              ║");
    println!("╚════════════════════════════════════════════╝");

    let (key_tx, key_rx) = std::sync::mpsc::channel::<EventType>();

    spawn_key_listener(key_tx);

    // Handle Ctrl-C gracefully.
    let running = Arc::new(AtomicBool::new(true));
    let ctrlc_flag = running.clone();
    ctrlc::set_handler(move || {
        ctrlc_flag.store(false, Ordering::SeqCst);
        println!("\n[screen-ocr] Shutting down…");
        std::process::exit(0);
    })?;

    let mut busy = false;
    let mut tts_child: Option<Child> = None;

    #[allow(unused_assignments)]
    loop {
        let event = match key_rx.recv() {
            Ok(e) => e,
            Err(_) => break,
        };

        match &event {
            // ── F9: Quick Capture (re-use stored region) ─────────────
            EventType::KeyPress(key)
                if *key == cfg.quick_capture_key && !busy =>
            {
                busy = true;

                // Load saved region; fall back to interactive selection
                let region = match load_region(&cfg.geometry_path) {
                    Ok(r) => {
                        println!(
                            "[screen-ocr] Quick capture: {}x{}+{}+{}",
                            r.w, r.h, r.x, r.y
                        );
                        r
                    }
                    Err(_) => {
                        println!("[screen-ocr] No saved region — select one now…");
                        match select_region(display) {
                            Ok(r) => {
                                if let Err(e) = save_region(&cfg.geometry_path, &r) {
                                    eprintln!("[screen-ocr] Could not save region: {e}");
                                }
                                println!(
                                    "[screen-ocr] Region saved: {}x{}+{}+{}",
                                    r.w, r.h, r.x, r.y
                                );
                                r
                            }
                            Err(e) => {
                                eprintln!("[screen-ocr] Selection error: {e}");
                                busy = false;
                                continue;
                            }
                        }
                    }
                };

                if let Err(e) = run_pipeline(display, &region, &cfg, &mut tts_child) {
                    eprintln!("[screen-ocr] Pipeline error: {e}");
                }

                busy = false;
                println!("[screen-ocr] Ready. F9=quick capture, F10=new region.");
            }

            // ── F10: Select New Region ───────────────────────────────
            EventType::KeyPress(key)
                if *key == cfg.select_region_key && !busy =>
            {
                busy = true;
                println!("[screen-ocr] Select a screen region…");

                match select_region(display) {
                    Ok(region) => {
                        if let Err(e) = save_region(&cfg.geometry_path, &region) {
                            eprintln!("[screen-ocr] Could not save region: {e}");
                        }
                        println!(
                            "[screen-ocr] Region saved: {}x{}+{}+{}",
                            region.w, region.h, region.x, region.y
                        );

                        if let Err(e) = run_pipeline(display, &region, &cfg, &mut tts_child) {
                            eprintln!("[screen-ocr] Pipeline error: {e}");
                        }
                    }
                    Err(e) => {
                        eprintln!("[screen-ocr] Selection error: {e}");
                    }
                }

                busy = false;
                println!("[screen-ocr] Ready. F9=quick capture, F10=new region.");
            }

            // ── F11: Stop TTS ─────────────────────────────────────
            EventType::KeyPress(key) if *key == cfg.stop_tts_key => {
                if let Some(ref mut child) = tts_child {
                    match child.try_wait() {
                        Ok(Some(_)) => {
                            println!("[screen-ocr] TTS already finished.");
                        }
                        _ => {
                            kill_tts(child);
                            println!("[screen-ocr] TTS stopped.");
                        }
                    }
                    tts_child = None;
                }
            }

            _ => {}
        }
    }

    Ok(())
}
