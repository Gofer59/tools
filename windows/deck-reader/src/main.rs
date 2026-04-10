// deck-reader — screen OCR + TTS (Linux SteamDeck; Windows port in progress)
//
// HOW IT WORKS
// ────────────
// A background thread (rdev) watches every key event globally.
// Alt + U  → interactive region select → OCR → clipboard → auto-speak
// Alt + I  → re-capture saved region   → OCR → clipboard → auto-speak
// Alt + Y  → TTS toggle (speak highlighted text, or stop if speaking)
//
// PLATFORM LAYER
// ──────────────
// All platform-specific IO (region select, screen capture, clipboard, text
// injection, TTS fallback, path helpers) lives in `src/platform/`, which
// re-exports the Linux or Windows backend at compile time via cfg. See
// `platform/mod.rs` for the shared API surface.
//
// THREADING MODEL
// ───────────────
//   main thread           ← state machine, blocking subprocess calls, dispatch
//   rdev listener thread  ← sends EventType messages over mpsc channel

mod platform;

use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[cfg(unix)]
use std::{
    io::{BufRead, BufReader},
    os::unix::net::UnixStream,
    os::unix::process::CommandExt,
    process::Stdio,
};

use anyhow::{Context, Result};
use rdev::{listen, Event, EventType, Key};
use serde::Deserialize;
use tempfile::NamedTempFile;

use crate::platform::Region;

// ─────────────────────────────────────────────────────────────────────────────
// Configuration — TOML schema
// ─────────────────────────────────────────────────────────────────────────────

/// Default config written to disk on first run.
const DEFAULT_CONFIG_TOML: &str = r#"[hotkeys]
# Key names: MetaLeft, KeyQ, F9, AltGr, ControlLeft, etc.
# Raw keycodes from Steam Input: "191" or "Unknown(191)"
# Combos: "MetaLeft+KeyQ" or "MetaLeft+191"
# Run `deck-reader --detect-keys` to discover keycodes.
tts_toggle  = "Alt+KeyY"
ocr_select  = "Alt+KeyU"
ocr_capture = "Alt+KeyI"

[tts]
voice = "en_US-lessac-medium"   # Piper model name (must exist in models dir)
speed = 1.0                     # 1.0=normal, 1.5=faster, 0.8=slower

[ocr]
language      = "eng"           # Tesseract lang codes: "eng", "eng+jpn", etc.
delivery_mode = "clipboard"     # "clipboard" | "type" | "both"
cleanup       = true            # clean OCR artifacts (stray symbols, repeated punct)

[paths]
# Optional overrides — defaults shown.
# models_dir  = "~/.local/share/deck-reader/models"
# venv_dir    = "~/.local/share/deck-reader/venv"
# region_file = "~/.local/share/deck-reader/last_region.json"
"#;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AppConfig {
    hotkeys: HotkeyConfig,
    tts:     TtsConfig,
    ocr:     OcrConfig,
    #[serde(default)]
    paths:   PathsConfig,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HotkeyConfig {
    tts_toggle:  String,
    ocr_select:  String,
    ocr_capture: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TtsConfig {
    voice: String,
    speed: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OcrConfig {
    language:      String,
    delivery_mode: String,
    #[serde(default = "default_true")]
    cleanup:       bool,
}

fn default_true() -> bool { true }

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct PathsConfig {
    models_dir:  Option<String>,
    venv_dir:    Option<String>,
    region_file: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// TTS daemon state
// ─────────────────────────────────────────────────────────────────────────────

/// Tracks a spawned TTS daemon process.
struct TtsDaemon {
    /// The daemon's process handle (None when reattaching to an orphaned daemon).
    process: Option<Child>,
    socket_path: PathBuf,
}

/// All TTS-related state: daemon connection, active playback, and fallback.
struct TtsState {
    daemon: Option<TtsDaemon>,
    /// Persistent socket connection to the daemon (Unix only).
    #[cfg(unix)]
    conn: Option<BufReader<UnixStream>>,
    /// Process-group ID of the current paplay process (Linux only — for instant kill).
    #[cfg(unix)]
    paplay_pgid: Option<i32>,
    /// True while audio is actively playing.
    speaking: bool,
    /// Fallback subprocess when the daemon is unavailable.
    fallback_child: Option<Child>,
}

impl TtsState {
    fn new() -> Self {
        Self {
            daemon: None,
            #[cfg(unix)]
            conn: None,
            #[cfg(unix)]
            paplay_pgid: None,
            speaking: false,
            fallback_child: None,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Config loading
// ─────────────────────────────────────────────────────────────────────────────

/// Load ~/.config/deck-reader/config.toml.
/// Auto-creates with defaults if the file does not exist.
/// Returns a clear error if the file is malformed.
fn load_config() -> Result<AppConfig> {
    let dir  = platform::config_dir();
    let path = dir.join("config.toml");

    if !path.exists() {
        fs::create_dir_all(&dir)
            .with_context(|| format!("Cannot create config directory {:?}", dir))?;
        fs::write(&path, DEFAULT_CONFIG_TOML)
            .with_context(|| format!("Cannot write default config to {:?}", path))?;
        println!("[deck-reader] Created default config at {:?}", path);
    }

    let text = fs::read_to_string(&path)
        .with_context(|| format!("Cannot read config file {:?}", path))?;

    toml::from_str(&text)
        .with_context(|| format!("Malformed config at {:?} — fix the TOML then restart", path))
}

/// Expand a path string's leading `~` via `shellexpand::tilde`.
/// Cross-platform: `$HOME` on Unix, `%USERPROFILE%` on Windows.
fn expand_path(s: &str) -> PathBuf {
    PathBuf::from(shellexpand::tilde(s).as_ref())
}

// ─────────────────────────────────────────────────────────────────────────────
// Hotkey parsing
// ─────────────────────────────────────────────────────────────────────────────

/// Map a config key-name string to an rdev Key variant.
///
/// Valid names: MetaLeft, MetaRight, AltLeft/Alt, AltRight/AltGr,
/// ShiftLeft, ShiftRight, ControlLeft/Control/Ctrl, ControlRight,
/// KeyA–KeyZ, F1–F12, Space, Return/Enter, Escape/Esc, Tab.
///
/// Also accepts raw keycodes for Steam Input virtual keys:
///   "191"          → Key::Unknown(191)
///   "Unknown(191)" → Key::Unknown(191)
fn str_to_key(s: &str) -> Result<Key> {
    match s.trim() {
        "MetaLeft"                       => Ok(Key::MetaLeft),
        "MetaRight"                      => Ok(Key::MetaRight),
        "AltLeft" | "Alt"                => Ok(Key::Alt),
        "AltRight" | "AltGr"            => Ok(Key::AltGr),
        "ShiftLeft"                      => Ok(Key::ShiftLeft),
        "ShiftRight"                     => Ok(Key::ShiftRight),
        "ControlLeft" | "Control" | "Ctrl" => Ok(Key::ControlLeft),
        "ControlRight"                   => Ok(Key::ControlRight),
        "KeyA" => Ok(Key::KeyA), "KeyB" => Ok(Key::KeyB), "KeyC" => Ok(Key::KeyC),
        "KeyD" => Ok(Key::KeyD), "KeyE" => Ok(Key::KeyE), "KeyF" => Ok(Key::KeyF),
        "KeyG" => Ok(Key::KeyG), "KeyH" => Ok(Key::KeyH), "KeyI" => Ok(Key::KeyI),
        "KeyJ" => Ok(Key::KeyJ), "KeyK" => Ok(Key::KeyK), "KeyL" => Ok(Key::KeyL),
        "KeyM" => Ok(Key::KeyM), "KeyN" => Ok(Key::KeyN), "KeyO" => Ok(Key::KeyO),
        "KeyP" => Ok(Key::KeyP), "KeyQ" => Ok(Key::KeyQ), "KeyR" => Ok(Key::KeyR),
        "KeyS" => Ok(Key::KeyS), "KeyT" => Ok(Key::KeyT), "KeyU" => Ok(Key::KeyU),
        "KeyV" => Ok(Key::KeyV), "KeyW" => Ok(Key::KeyW), "KeyX" => Ok(Key::KeyX),
        "KeyY" => Ok(Key::KeyY), "KeyZ" => Ok(Key::KeyZ),
        "F1"  => Ok(Key::F1),  "F2"  => Ok(Key::F2),  "F3"  => Ok(Key::F3),
        "F4"  => Ok(Key::F4),  "F5"  => Ok(Key::F5),  "F6"  => Ok(Key::F6),
        "F7"  => Ok(Key::F7),  "F8"  => Ok(Key::F8),  "F9"  => Ok(Key::F9),
        "F10" => Ok(Key::F10), "F11" => Ok(Key::F11), "F12" => Ok(Key::F12),
        "Backspace"           => Ok(Key::Backspace),
        "CapsLock"            => Ok(Key::CapsLock),
        "Delete"              => Ok(Key::Delete),
        "DownArrow"           => Ok(Key::DownArrow),
        "End"                 => Ok(Key::End),
        "Escape" | "Esc"      => Ok(Key::Escape),
        "Home"                => Ok(Key::Home),
        "Insert"              => Ok(Key::Insert),
        "LeftArrow"           => Ok(Key::LeftArrow),
        "NumLock"             => Ok(Key::NumLock),
        "PageDown"            => Ok(Key::PageDown),
        "PageUp"              => Ok(Key::PageUp),
        "Pause"               => Ok(Key::Pause),
        "PrintScreen"         => Ok(Key::PrintScreen),
        "Return" | "Enter"    => Ok(Key::Return),
        "RightArrow"          => Ok(Key::RightArrow),
        "ScrollLock"          => Ok(Key::ScrollLock),
        "Space"               => Ok(Key::Space),
        "Tab"                 => Ok(Key::Tab),
        "UpArrow"             => Ok(Key::UpArrow),
        other => {
            // "Unknown(191)" syntax
            if let Some(inner) = other.strip_prefix("Unknown(").and_then(|s| s.strip_suffix(')')) {
                let code: u32 = inner.trim().parse()
                    .with_context(|| format!("Invalid raw keycode in {:?}", other))?;
                return Ok(Key::Unknown(code));
            }
            // Bare integer "191" — raw keycode from Steam Input
            if let Ok(code) = other.parse::<u32>() {
                return Ok(Key::Unknown(code));
            }
            anyhow::bail!(
                "Unknown key name {:?}. Valid: named keys (MetaLeft, F9, KeyQ…), \
                 raw keycodes (191), or Unknown(191). \
                 Run `deck-reader --detect-keys` to discover keycodes.",
                other
            );
        }
    }
}

/// Map an rdev Key variant to its raw X11 keycode.
///
/// Mirrors rdev's internal `code_from_key()` (which is in a private module).
/// Used by `--detect-keys` mode and the startup banner.
fn key_to_raw_code(key: Key) -> Option<u32> {
    match key {
        Key::Alt          => Some(64),
        Key::AltGr        => Some(108),
        Key::Backspace    => Some(22),
        Key::CapsLock     => Some(66),
        Key::ControlLeft  => Some(37),
        Key::ControlRight => Some(105),
        Key::Delete       => Some(119),
        Key::DownArrow    => Some(116),
        Key::End          => Some(115),
        Key::Escape       => Some(9),
        Key::F1           => Some(67),
        Key::F2           => Some(68),
        Key::F3           => Some(69),
        Key::F4           => Some(70),
        Key::F5           => Some(71),
        Key::F6           => Some(72),
        Key::F7           => Some(73),
        Key::F8           => Some(74),
        Key::F9           => Some(75),
        Key::F10          => Some(76),
        Key::F11          => Some(95),
        Key::F12          => Some(96),
        Key::Home         => Some(110),
        Key::LeftArrow    => Some(113),
        Key::MetaLeft     => Some(133),
        Key::MetaRight    => Some(134),
        Key::PageDown     => Some(117),
        Key::PageUp       => Some(112),
        Key::Return       => Some(36),
        Key::RightArrow   => Some(114),
        Key::ShiftLeft    => Some(50),
        Key::ShiftRight   => Some(62),
        Key::Space        => Some(65),
        Key::Tab          => Some(23),
        Key::UpArrow      => Some(111),
        Key::PrintScreen  => Some(107),
        Key::ScrollLock   => Some(78),
        Key::Pause        => Some(127),
        Key::NumLock      => Some(77),
        Key::BackQuote    => Some(49),
        Key::Num1         => Some(10),
        Key::Num2         => Some(11),
        Key::Num3         => Some(12),
        Key::Num4         => Some(13),
        Key::Num5         => Some(14),
        Key::Num6         => Some(15),
        Key::Num7         => Some(16),
        Key::Num8         => Some(17),
        Key::Num9         => Some(18),
        Key::Num0         => Some(19),
        Key::Minus        => Some(20),
        Key::Equal        => Some(21),
        Key::KeyQ         => Some(24),
        Key::KeyW         => Some(25),
        Key::KeyE         => Some(26),
        Key::KeyR         => Some(27),
        Key::KeyT         => Some(28),
        Key::KeyY         => Some(29),
        Key::KeyU         => Some(30),
        Key::KeyI         => Some(31),
        Key::KeyO         => Some(32),
        Key::KeyP         => Some(33),
        Key::LeftBracket  => Some(34),
        Key::RightBracket => Some(35),
        Key::KeyA         => Some(38),
        Key::KeyS         => Some(39),
        Key::KeyD         => Some(40),
        Key::KeyF         => Some(41),
        Key::KeyG         => Some(42),
        Key::KeyH         => Some(43),
        Key::KeyJ         => Some(44),
        Key::KeyK         => Some(45),
        Key::KeyL         => Some(46),
        Key::SemiColon    => Some(47),
        Key::Quote        => Some(48),
        Key::BackSlash    => Some(51),
        Key::IntlBackslash => Some(94),
        Key::KeyZ         => Some(52),
        Key::KeyX         => Some(53),
        Key::KeyC         => Some(54),
        Key::KeyV         => Some(55),
        Key::KeyB         => Some(56),
        Key::KeyN         => Some(57),
        Key::KeyM         => Some(58),
        Key::Comma        => Some(59),
        Key::Dot          => Some(60),
        Key::Slash        => Some(61),
        Key::Insert       => Some(118),
        Key::KpReturn     => Some(104),
        Key::KpMinus      => Some(82),
        Key::KpPlus       => Some(86),
        Key::KpMultiply   => Some(63),
        Key::KpDivide     => Some(106),
        Key::Kp0          => Some(90),
        Key::Kp1          => Some(87),
        Key::Kp2          => Some(88),
        Key::Kp3          => Some(89),
        Key::Kp4          => Some(83),
        Key::Kp5          => Some(84),
        Key::Kp6          => Some(85),
        Key::Kp7          => Some(79),
        Key::Kp8          => Some(80),
        Key::Kp9          => Some(81),
        Key::KpDelete     => Some(91),
        Key::Unknown(code) => Some(code),
        _                 => None,
    }
}

/// Format a key for display: "MetaLeft" or "Unknown(191)".
fn format_key(key: Key) -> String {
    match key {
        Key::Unknown(code) => format!("Unknown({code})"),
        named => format!("{named:?}"),
    }
}

/// Format a parsed hotkey for the startup banner.
/// Returns (config_str, code_str) e.g. ("MetaLeft+KeyQ", "133+24").
fn format_hotkey_banner(hk: (Option<Key>, Key)) -> (String, String) {
    let name = match hk.0 {
        Some(m) => format!("{}+{}", format_key(m), format_key(hk.1)),
        None    => format_key(hk.1),
    };
    let code = match hk.0 {
        Some(m) => format!(
            "{}+{}",
            key_to_raw_code(m).map_or("?".into(), |c| c.to_string()),
            key_to_raw_code(hk.1).map_or("?".into(), |c| c.to_string()),
        ),
        None => key_to_raw_code(hk.1).map_or("?".into(), |c| c.to_string()),
    };
    (name, code)
}

/// Parse "MetaLeft+KeyQ" → (Some(MetaLeft), KeyQ)
/// Parse "F9"            → (None, F9)
fn parse_hotkey(s: &str) -> Result<(Option<Key>, Key)> {
    match s.splitn(2, '+').collect::<Vec<_>>().as_slice() {
        [single] => Ok((None, str_to_key(single.trim())?)),
        [modifier, action] => Ok((
            Some(str_to_key(modifier.trim())?),
            str_to_key(action.trim())?,
        )),
        _ => anyhow::bail!("Invalid hotkey string: {:?}", s),
    }
}

/// True if key `k` was just pressed and the hotkey's modifier (if any) is held.
fn hotkey_matches(k: Key, hk: (Option<Key>, Key), held: &HashSet<Key>) -> bool {
    k == hk.1 && hk.0.is_none_or(|m| held.contains(&m))
}

// ─────────────────────────────────────────────────────────────────────────────
// Region geometry — persistence
// (the Region struct itself lives in platform::mod.rs as a shared type)
// ─────────────────────────────────────────────────────────────────────────────

fn load_region(path: &Path) -> Result<Region> {
    let contents = fs::read_to_string(path).with_context(|| {
        format!(
            "No saved region at {:?} — use Alt+U first to draw one",
            path
        )
    })?;
    serde_json::from_str(&contents).context("Failed to parse saved region JSON")
}

fn save_region(path: &Path, region: &Region) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Cannot create directory {:?}", parent))?;
    }
    let json = serde_json::to_string_pretty(region)?;
    fs::write(path, json).with_context(|| format!("Cannot write region to {:?}", path))
}

// ─────────────────────────────────────────────────────────────────────────────
// OCR extraction
// ─────────────────────────────────────────────────────────────────────────────

/// Call ocr_extract_wrapper.sh with the image path and Tesseract lang code.
/// Returns the extracted text from stdout.
fn ocr_extract(image_path: &Path, ocr_wrapper: &Path, lang: &str, cleanup: bool) -> Result<String> {
    println!("[deck-reader] Running OCR (lang={lang})…");

    let cleanup_flag = if cleanup { "cleanup" } else { "raw" };
    let output = Command::new(ocr_wrapper)
        .arg(image_path)
        .arg(lang)
        .arg(cleanup_flag)
        .output()
        .with_context(|| {
            format!(
                "Failed to run OCR script at {:?}. Did you run install.sh?",
                ocr_wrapper
            )
        })?;

    // Echo Python's stderr (diagnostics) to our stderr.
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        eprint!("{}", stderr);
    }

    if !output.status.success() {
        anyhow::bail!("OCR script failed:\n{}", stderr);
    }

    Ok(String::from_utf8(output.stdout)
        .context("OCR output was not valid UTF-8")?
        .trim()
        .to_owned())
}

// ─────────────────────────────────────────────────────────────────────────────
// Text delivery
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum DeliveryMode {
    Clipboard,
    Type,
    Both,
}

fn parse_delivery_mode(s: &str) -> Result<DeliveryMode> {
    match s.trim().to_lowercase().as_str() {
        "clipboard" => Ok(DeliveryMode::Clipboard),
        "type"      => Ok(DeliveryMode::Type),
        "both"      => Ok(DeliveryMode::Both),
        other       => anyhow::bail!(
            "Invalid delivery_mode {:?} in config.toml. Valid: \"clipboard\", \"type\", \"both\"",
            other
        ),
    }
}

fn deliver_text(text: &str, mode: DeliveryMode) -> Result<()> {
    if text.is_empty() {
        println!("[deck-reader] No text extracted.");
        return Ok(());
    }

    match mode {
        DeliveryMode::Clipboard => {
            platform::copy_to_clipboard(text)?;
            println!("[deck-reader] Copied {} chars to clipboard.", text.len());
        }
        DeliveryMode::Type => {
            platform::type_text(text)?;
            println!("[deck-reader] Typed {} chars at cursor.", text.len());
        }
        DeliveryMode::Both => {
            platform::copy_to_clipboard(text)?;
            platform::type_text(text)?;
            println!("[deck-reader] Copied + typed {} chars.", text.len());
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// TTS daemon management (Linux only — daemon uses Unix sockets + setsid)
// ─────────────────────────────────────────────────────────────────────────────

/// Ensure the TTS daemon is running and `state.daemon` is set.
///
/// On first call, spawns the Python daemon process and waits for its READY
/// signal.  On subsequent calls, checks if the existing daemon is still alive,
/// or tries to reconnect to a surviving daemon from a previous session.
#[cfg(unix)]
fn ensure_daemon(
    state: &mut TtsState,
    venv_python: &Path,
    daemon_script: &Path,
    models_dir: &Path,
) -> Result<()> {
    // Fast path: daemon already tracked and alive.
    if let Some(ref mut d) = state.daemon {
        let alive = match d.process {
            Some(ref mut p) => matches!(p.try_wait(), Ok(None)),
            None => true, // Reattached orphan — assume alive (socket will fail if not).
        };
        if alive {
            return Ok(());
        }
        // Daemon exited — clear state and try to respawn below.
        println!("[deck-reader] TTS daemon exited unexpectedly, restarting…");
        state.daemon = None;
        state.conn = None;
    }

    let socket_path = platform::tts_socket_path();

    // Try connecting to an existing daemon (survives deck-reader restarts).
    if socket_path.exists() {
        if let Ok(stream) = UnixStream::connect(&socket_path) {
            println!("[deck-reader] Reattached to existing TTS daemon.");
            stream.set_nonblocking(false)?;
            state.conn = Some(BufReader::new(stream));
            // We don't have the Child handle for the orphaned daemon, so we
            // store a placeholder.  socket_path is enough for cleanup.
            state.daemon = Some(TtsDaemon {
                process: None, // No Child handle for an orphaned daemon.
                socket_path: socket_path.clone(),
            });
            return Ok(());
        }
        // Stale socket — remove it.
        let _ = fs::remove_file(&socket_path);
    }

    if !daemon_script.exists() {
        anyhow::bail!(
            "TTS daemon script not found at {:?}. Did you run install.sh?",
            daemon_script
        );
    }

    println!("[deck-reader] Starting TTS daemon…");

    // SAFETY: setsid() is async-signal-safe.
    let mut child = unsafe {
        Command::new(venv_python)
            .arg(daemon_script)
            .arg(&socket_path)
            .arg(models_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            })
            .spawn()
            .with_context(|| {
                format!(
                    "Failed to start TTS daemon. venv={:?} script={:?}",
                    venv_python, daemon_script
                )
            })?
    };

    // Wait for "READY" on stdout with an enforceable timeout.
    // We read in a separate thread because read_line() blocks and cannot be
    // interrupted by a deadline check in the same thread.
    let stdout = child
        .stdout
        .take()
        .context("No stdout from TTS daemon")?;

    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<bool>();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => { let _ = ready_tx.send(false); return; }
                Ok(_) => {
                    if line.trim() == "READY" {
                        let _ = ready_tx.send(true);
                        return;
                    }
                }
            }
        }
    });

    match ready_rx.recv_timeout(std::time::Duration::from_secs(15)) {
        Ok(true) => {}
        _ => {
            let pid = child.id() as i32;
            unsafe { libc::kill(-pid, libc::SIGKILL); }
            let _ = child.wait();
            anyhow::bail!("TTS daemon did not become ready within 15 seconds");
        }
    }

    println!("[deck-reader] TTS daemon ready (pid={}).", child.id());

    state.daemon = Some(TtsDaemon {
        process: Some(child),
        socket_path,
    });

    Ok(())
}

/// Ensure we have an active socket connection to the daemon.
#[cfg(unix)]
fn ensure_connection(state: &mut TtsState) -> Result<()> {
    if state.conn.is_some() {
        return Ok(());
    }
    let socket_path = state
        .daemon
        .as_ref()
        .map(|d| &d.socket_path)
        .context("No daemon to connect to")?;

    let stream = UnixStream::connect(socket_path)
        .with_context(|| format!("Cannot connect to TTS daemon at {:?}", socket_path))?;
    state.conn = Some(BufReader::new(stream));
    Ok(())
}

/// Send a TTS request to the daemon.  Falls back to the cold-start wrapper
/// if the daemon is unreachable.
#[allow(clippy::too_many_arguments, unused_variables)]
fn request_tts(
    text: &str,
    voice: &str,
    speed: f32,
    state: &mut TtsState,
    venv_python: &Path,
    daemon_script: &Path,
    models_dir: &Path,
    tts_wrapper: &Path,
) -> Result<()> {
    let t0 = std::time::Instant::now();
    println!(
        "[deck-reader] Speaking {} chars (voice={voice}, speed={speed})…",
        text.len()
    );

    // If currently speaking, stop first.
    stop_tts(state);

    // ── Daemon path (Linux only) ──────────────────────────────────────────────
    #[cfg(unix)]
    {
        if let Err(e) = ensure_daemon(state, venv_python, daemon_script, models_dir) {
            eprintln!("[deck-reader] Daemon unavailable ({e}), using fallback.");
            let child = platform::spawn_tts_fallback(text, voice, speed, tts_wrapper)?;
            state.fallback_child = Some(child);
            state.speaking = true;
            return Ok(());
        }
        println!("[deck-reader] TIMING ensure_daemon={}ms", t0.elapsed().as_millis());

        if let Err(e) = ensure_connection(state) {
            eprintln!("[deck-reader] Cannot connect to daemon ({e}), using fallback.");
            state.daemon = None;
            state.conn = None;
            let child = platform::spawn_tts_fallback(text, voice, speed, tts_wrapper)?;
            state.fallback_child = Some(child);
            state.speaking = true;
            return Ok(());
        }

        // Build and send the JSON request.
        let request = serde_json::json!({
            "text": text,
            "voice": voice,
            "speed": speed,
        });
        let request_line = format!("{}\n", request);

        let conn = state.conn.as_mut().unwrap();
        if let Err(e) = conn.get_mut().write_all(request_line.as_bytes()) {
            eprintln!("[deck-reader] Socket write failed ({e}), resetting daemon connection.");
            state.conn = None;
            // One retry: reconnect and resend.
            if ensure_connection(state).is_ok() {
                let conn = state.conn.as_mut().unwrap();
                if let Err(e2) = conn.get_mut().write_all(request_line.as_bytes()) {
                    eprintln!("[deck-reader] Socket write retry failed ({e2}), using fallback.");
                    state.conn = None;
                    let child = platform::spawn_tts_fallback(text, voice, speed, tts_wrapper)?;
                    state.fallback_child = Some(child);
                    state.speaking = true;
                    return Ok(());
                }
            } else {
                state.daemon = None;
                let child = platform::spawn_tts_fallback(text, voice, speed, tts_wrapper)?;
                state.fallback_child = Some(child);
                state.speaking = true;
                return Ok(());
            }
        }

        // Read the daemon's response (blocking — should arrive quickly).
        let conn = state.conn.as_mut().unwrap();
        conn.get_ref().set_nonblocking(false)?;
        let mut response_line = String::new();
        conn.read_line(&mut response_line)?;

        let resp: serde_json::Value = serde_json::from_str(response_line.trim())
            .with_context(|| format!("Bad daemon response: {:?}", response_line))?;

        match resp.get("status").and_then(|s| s.as_str()) {
            Some("playing") => {
                if let Some(pgid) = resp.get("pgid").and_then(|p| p.as_i64()) {
                    state.paplay_pgid = Some(pgid as i32);
                }
                state.speaking = true;
                println!(
                    "[deck-reader] TIMING request_tts → playing in {}ms",
                    t0.elapsed().as_millis()
                );
                // Switch to non-blocking so the event loop can poll for "done".
                conn.get_ref().set_nonblocking(true)?;
            }
            Some("done") => {
                // Empty text or instant completion — nothing playing.
                state.speaking = false;
            }
            Some("error") => {
                let msg = resp
                    .get("msg")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown");
                anyhow::bail!("TTS daemon error: {msg}");
            }
            _ => {
                anyhow::bail!("Unexpected daemon response: {response_line}");
            }
        }

        Ok(())
    }

    // ── Fallback path (Windows — daemon not supported in MVP) ─────────────────
    #[cfg(windows)]
    {
        let child = platform::spawn_tts_fallback(text, voice, speed, tts_wrapper)?;
        state.fallback_child = Some(child);
        state.speaking = true;
        Ok(())
    }
}

/// Poll the daemon socket for a "done" message (non-blocking).
/// Returns true if TTS finished playing.
fn poll_tts_done(state: &mut TtsState) -> bool {
    // Daemon path: non-blocking read (Linux only).
    #[cfg(unix)]
    if state.speaking && state.conn.is_some() && state.fallback_child.is_none() {
        let conn = state.conn.as_mut().unwrap();
        let mut line = String::new();
        match conn.read_line(&mut line) {
            Ok(0) => {
                // EOF — daemon disconnected.
                state.speaking = false;
                state.paplay_pgid = None;
                state.conn = None;
                return true;
            }
            Ok(_) => {
                if let Ok(resp) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                    if resp.get("status").and_then(|s| s.as_str()) == Some("done")
                        || resp.get("status").and_then(|s| s.as_str()) == Some("error")
                    {
                        state.speaking = false;
                        state.paplay_pgid = None;
                        return true;
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Nothing available yet — TTS still playing.
            }
            Err(_) => {
                // Read error — assume daemon died.
                state.speaking = false;
                state.paplay_pgid = None;
                state.conn = None;
                return true;
            }
        }
    }

    // Fallback path: poll child process.
    if let Some(ref mut child) = state.fallback_child {
        if let Ok(Some(_)) = child.try_wait() {
            state.fallback_child = None;
            state.speaking = false;
            return true;
        }
    }

    false
}

/// Stop any active TTS playback immediately.
fn stop_tts(state: &mut TtsState) {
    // Daemon path: kill paplay process group (Linux only).
    #[cfg(unix)]
    if let Some(pgid) = state.paplay_pgid.take() {
        println!("[deck-reader] Stopping TTS…");
        unsafe { libc::kill(-pgid, libc::SIGKILL); }
        state.speaking = false;

        // Drain any stale responses from the socket so the next request_tts()
        // starts with a clean buffer.  The daemon may or may not send a "done"
        // after detecting the broken pipe — we discard whatever is there.
        if let Some(ref mut conn) = state.conn {
            let _ = conn.get_ref().set_nonblocking(true);
            let mut junk = String::new();
            while let Ok(n) = conn.read_line(&mut junk) {
                if n == 0 { break; }
                junk.clear();
            }
        }
    }

    // Fallback path: kill the subprocess (cross-platform).
    if let Some(ref mut child) = state.fallback_child.take() {
        platform::kill_tts_fallback(child);
        state.speaking = false;
    }
}

/// Clean up the daemon on program exit.
fn shutdown_daemon(state: &mut TtsState) {
    stop_tts(state);

    // Daemon teardown (Linux only).
    #[cfg(unix)]
    {
        // Send graceful shutdown command.
        if let Some(ref mut conn) = state.conn {
            let _ = conn.get_mut().set_nonblocking(false);
            let _ = conn.get_mut().write_all(b"{\"cmd\":\"shutdown\"}\n");
        }
        state.conn = None;

        if let Some(ref mut daemon) = state.daemon {
            // Give it 500ms to exit gracefully, then force-kill.
            std::thread::sleep(std::time::Duration::from_millis(500));
            if let Some(ref mut process) = daemon.process {
                if matches!(process.try_wait(), Ok(None)) {
                    let pid = process.id() as i32;
                    unsafe { libc::kill(-pid, libc::SIGKILL); }
                    let _ = process.wait();
                }
            }
            let _ = fs::remove_file(&daemon.socket_path);
        }
    }
    state.daemon = None;
}

// ─────────────────────────────────────────────────────────────────────────────
// Key event listener
// ─────────────────────────────────────────────────────────────────────────────

/// Spawn the rdev key listener in a background thread.
/// All KeyPress and KeyRelease events are forwarded over `tx`.
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
            eprintln!("[deck-reader] rdev listener error: {:?}", e);
            eprintln!("[deck-reader] Is the current user in the 'input' group?");
            eprintln!("[deck-reader] Run: sudo usermod -aG input $USER  (then reboot)");
        }
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Key discovery mode (--detect-keys)
// ─────────────────────────────────────────────────────────────────────────────

/// Interactive key discovery: prints every key event with its raw code.
/// Used to find Steam Input virtual keycodes for config.toml.
fn run_detect_keys() -> Result<()> {
    println!("[deck-reader] Key discovery mode — press keys to see their codes. Ctrl-C to exit.");
    println!("[deck-reader] Use the printed codes in ~/.config/deck-reader/config.toml");
    println!();

    let running = Arc::new(AtomicBool::new(true));
    let ctrlc_flag = running.clone();
    ctrlc::set_handler(move || {
        ctrlc_flag.store(false, Ordering::SeqCst);
    })?;

    let running2 = running.clone();
    if let Err(e) = listen(move |event: Event| {
        if !running2.load(Ordering::SeqCst) {
            return;
        }

        let (label, key) = match event.event_type {
            EventType::KeyPress(k)  => ("Press  ", k),
            EventType::KeyRelease(k) => ("Release", k),
            _ => return,
        };

        let name = format_key(key);
        let code_str = key_to_raw_code(key)
            .map_or("?".into(), |c: u32| c.to_string());

        match key {
            Key::Unknown(_) => {
                println!(
                    "[deck-reader]   {label}: {name:<20} (code: {code_str:<5})  \
                     ← use \"{code_str}\" in config.toml"
                );
            }
            _ => {
                println!(
                    "[deck-reader]   {label}: {name:<20} (code: {code_str:<5})"
                );
            }
        }
    }) {
        eprintln!("[deck-reader] rdev listener error: {e:?}");
        eprintln!("[deck-reader] Is the current user in the 'input' group?");
        eprintln!("[deck-reader] Run: sudo usermod -aG input $USER  (then reboot)");
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// OCR + TTS pipeline
// ─────────────────────────────────────────────────────────────────────────────

/// Capture region → OCR → deliver text → auto-speak.
#[allow(clippy::too_many_arguments)]
fn run_ocr_pipeline(
    region:         &Region,
    ocr_wrapper:    &Path,
    lang:           &str,
    cleanup:        bool,
    delivery_mode:  DeliveryMode,
    voice:          &str,
    speed:          f32,
    tts_state:      &mut TtsState,
    venv_python:    &Path,
    daemon_script:  &Path,
    models_dir:     &Path,
    tts_wrapper:    &Path,
) -> Result<usize> {
    // 1. Capture region to a temp PNG (auto-deleted when `tmp` drops).
    let tmp = NamedTempFile::with_suffix(".png")
        .context("Could not create temporary PNG file")?;
    platform::capture_region(region, tmp.path())?;

    // 2. OCR
    let text = ocr_extract(tmp.path(), ocr_wrapper, lang, cleanup)?;
    let char_count = text.len();

    // 3. Deliver text
    deliver_text(&text, delivery_mode)?;

    // 4. TTS (non-blocking — stop previous if still playing, then start new)
    if !text.is_empty() {
        if let Err(e) = request_tts(
            &text, voice, speed, tts_state,
            venv_python, daemon_script, models_dir, tts_wrapper,
        ) {
            eprintln!("[deck-reader] TTS error (continuing without speech): {e}");
        }
    }

    Ok(char_count)
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    // ── CLI flags ────────────────────────────────────────────────────────────
    if std::env::args().any(|a| a == "--detect-keys") {
        return run_detect_keys();
    }

    // ── Load and validate config ─────────────────────────────────────────────
    let cfg = load_config()?;

    // ── Parse hotkeys ────────────────────────────────────────────────────────
    let hk_tts = parse_hotkey(&cfg.hotkeys.tts_toggle)
        .with_context(|| format!("Invalid tts_toggle: {:?}", cfg.hotkeys.tts_toggle))?;
    let hk_select = parse_hotkey(&cfg.hotkeys.ocr_select)
        .with_context(|| format!("Invalid ocr_select: {:?}", cfg.hotkeys.ocr_select))?;
    let hk_capture = parse_hotkey(&cfg.hotkeys.ocr_capture)
        .with_context(|| format!("Invalid ocr_capture: {:?}", cfg.hotkeys.ocr_capture))?;

    // ── Resolve paths ────────────────────────────────────────────────────────
    let region_file = cfg
        .paths
        .region_file
        .as_deref()
        .map(expand_path)
        .unwrap_or_else(|| platform::data_dir().join("last_region.json"));

    let exe      = std::env::current_exe().unwrap_or_default();
    let bin_dir  = exe.parent().unwrap_or_else(|| Path::new("."));
    #[cfg(unix)]
    let tts_wrapper = bin_dir.join("tts_speak_wrapper.sh");
    #[cfg(windows)]
    let tts_wrapper = bin_dir.join("tts_speak_wrapper.bat"); // unused on Windows (spawn_tts_fallback ignores it)

    #[cfg(unix)]
    let ocr_wrapper = bin_dir.join("ocr_extract_wrapper.sh");
    #[cfg(windows)]
    let ocr_wrapper = bin_dir.join("ocr_extract_wrapper.bat");

    let daemon_script = bin_dir.join("tts_daemon.py");
    let venv_python   = cfg.paths.venv_dir.as_deref()
        .map(|s| expand_path(s).join("bin/python3"))
        .unwrap_or_else(|| platform::data_dir().join("venv/bin/python3"));
    let models_dir    = cfg.paths.models_dir.as_deref()
        .map(expand_path)
        .unwrap_or_else(|| platform::data_dir().join("models"));

    let delivery_mode = parse_delivery_mode(&cfg.ocr.delivery_mode)?;
    let has_region    = region_file.exists();

    // ── Startup banner ───────────────────────────────────────────────────────
    let (sel_name,  sel_code)  = format_hotkey_banner(hk_select);
    let (cap_name,  cap_code)  = format_hotkey_banner(hk_capture);
    let (tts_name,  tts_code)  = format_hotkey_banner(hk_tts);

    println!("╔══════════════════════════════════════════════════════╗");
    println!("║               deck-reader  ready                     ║");
    println!("╠══════════════════════════════════════════════════════╣");
    println!("║  Hotkeys:                                             ║");
    println!("║    OCR select  : {:<24} ({:<8}) ║", sel_name, sel_code);
    println!("║    OCR capture : {:<24} ({:<8}) ║", cap_name, cap_code);
    println!("║    TTS toggle  : {:<24} ({:<8}) ║", tts_name, tts_code);
    println!("╠══════════════════════════════════════════════════════╣");
    println!("║  TTS voice  : {:<38}║", cfg.tts.voice);
    println!("║  TTS speed  : {:<38}║", cfg.tts.speed);
    println!("║  OCR lang   : {:<38}║", cfg.ocr.language);
    println!("║  Delivery   : {:<38}║", cfg.ocr.delivery_mode);
    println!("║  Display    : {:<38}║", platform::backend_description());
    println!("╠══════════════════════════════════════════════════════╣");
    println!("║  Region : {:<42}║",
        if has_region { "saved (ready for Alt+I)" } else { "none — use Alt+U first" });
    #[cfg(unix)]
    println!("║  TTS dmn: {:<42}║",
        if daemon_script.exists() { "found (fast path)" } else { "MISSING — fallback mode" });
    #[cfg(windows)]
    println!("║  TTS dmn: {:<42}║", "not supported (Windows MVP — fallback only)");
    println!("║  TTS wrp: {:<42}║",
        if tts_wrapper.exists() { "found" } else { "MISSING — run install.sh" });
    println!("║  OCR wrp: {:<42}║",
        if ocr_wrapper.exists() { "found" } else { "MISSING — run install.sh" });
    println!("╠══════════════════════════════════════════════════════╣");
    println!("║  Config: ~/.config/deck-reader/config.toml            ║");
    println!("║  Ctrl-C to quit                                       ║");
    println!("╚══════════════════════════════════════════════════════╝");

    // ── GUI window (Linux only) ──────────────────────────────────────────────
    // Spawn a Tk status window on Linux. Skipped on Windows (deferred to phase 2).
    let mut gui_stdin: Option<std::process::ChildStdin> = None;
    #[allow(unused_mut)] // mutated inside #[cfg(unix)] block
    let mut gui_child: Option<Child> = None;
    #[cfg(unix)]
    {
        let gui_script = bin_dir.join("gui_window.py");
        if gui_script.exists() {
            match Command::new(&venv_python)
                .arg(&gui_script)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(mut child) => {
                    gui_stdin = child.stdin.take();
                    gui_child = Some(child);
                    println!("[deck-reader] GUI window launched.");
                }
                Err(e) => {
                    eprintln!("[deck-reader] Could not launch GUI (continuing headless): {e}");
                }
            }
        }
    }

    // Helper: send a status line to the GUI window (no-op if GUI not running).
    macro_rules! gui_send {
        ($stdin:expr, $($arg:tt)*) => {
            if let Some(ref mut w) = $stdin {
                let _ = writeln!(w, $($arg)*);
                let _ = w.flush();
            }
        };
    }

    // ── Key listener ─────────────────────────────────────────────────────────
    let (key_tx, key_rx) = std::sync::mpsc::channel::<EventType>();
    spawn_key_listener(key_tx);

    // ── Ctrl-C / SIGTERM handler ─────────────────────────────────────────────
    // Sets running=false; main loop uses recv_timeout so it can check and exit
    // cleanly (killing any active TTS subprocess first).
    // The GUI's Quit button sends SIGTERM, which ctrlc also catches.
    let running     = Arc::new(AtomicBool::new(true));
    let ctrlc_flag  = running.clone();
    ctrlc::set_handler(move || {
        ctrlc_flag.store(false, Ordering::SeqCst);
        println!("\n[deck-reader] Shutting down…");
    })?;

    // ── State ────────────────────────────────────────────────────────────────
    let mut tts_state = TtsState::new();
    let mut held: HashSet<Key>       = HashSet::new();
    // `busy` is true while an OCR pipeline is blocking the main thread.
    // Alt+U and Alt+I are suppressed while busy; Alt+Y (stop TTS) is not.
    let mut busy = false;

    // ── Event loop ───────────────────────────────────────────────────────────
    #[allow(unused_assignments)]
    loop {
        // Check Ctrl-C flag before blocking.
        if !running.load(Ordering::SeqCst) {
            break;
        }

        // Poll for natural TTS exit so the toggle behaves correctly.
        // This runs every iteration (including timeouts) so completion is
        // detected even when no keys are pressed.
        if tts_state.speaking && poll_tts_done(&mut tts_state) {
            gui_send!(gui_stdin, "tts:idle");
            println!("[deck-reader] TTS finished naturally.");
        }

        // Use recv_timeout so we can periodically check the running flag.
        let event = match key_rx.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(e)  => e,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        };

        match event {
            // ── Track all held keys (for modifier detection) ─────────────
            EventType::KeyRelease(k) => {
                held.remove(&k);
            }

            EventType::KeyPress(k) => {
                held.insert(k);

                // ── Alt+U: interactive OCR select ───────────────────────
                if hotkey_matches(k, hk_select, &held) && !busy {
                    busy = true;
                    println!("[deck-reader] Select a screen region (draw a rectangle)…");

                    gui_send!(gui_stdin, "status:Selecting region...");
                    match platform::select_region() {
                        Ok(region) => {
                            if let Err(e) = save_region(&region_file, &region) {
                                eprintln!("[deck-reader] Could not save region: {e}");
                            } else {
                                println!(
                                    "[deck-reader] Region saved: {}x{}+{}+{}",
                                    region.w, region.h, region.x, region.y
                                );
                            }
                            gui_send!(gui_stdin, "status:Running OCR...");
                            match run_ocr_pipeline(
                                &region,
                                &ocr_wrapper,
                                &cfg.ocr.language,
                                cfg.ocr.cleanup,
                                delivery_mode,
                                &cfg.tts.voice,
                                cfg.tts.speed,
                                &mut tts_state,
                                &venv_python,
                                &daemon_script,
                                &models_dir,
                                &tts_wrapper,
                            ) {
                                Ok(n) => {
                                    gui_send!(gui_stdin, "ocr:{n} chars");
                                    gui_send!(gui_stdin, "tts:speaking");
                                }
                                Err(e) => eprintln!("[deck-reader] Pipeline error: {e}"),
                            }
                        }
                        Err(e) => eprintln!("[deck-reader] Selection cancelled: {e}"),
                    }

                    // Drain queued events to clear stale held-key state
                    // (user may have released Meta during the blocking pipeline).
                    while let Ok(ev) = key_rx.try_recv() {
                        match ev {
                            EventType::KeyPress(k)   => { held.insert(k); }
                            EventType::KeyRelease(k) => { held.remove(&k); }
                            _ => {}
                        }
                    }
                    busy = false;
                    gui_send!(gui_stdin, "status:Listening...");
                    println!("[deck-reader] Ready.");
                }
                // ── Alt+I: re-capture saved region ─────────────────────
                else if hotkey_matches(k, hk_capture, &held) && !busy {
                    busy = true;
                    gui_send!(gui_stdin, "status:Running OCR...");

                    match load_region(&region_file) {
                        Ok(region) => {
                            println!(
                                "[deck-reader] Re-capturing {}x{}+{}+{}…",
                                region.w, region.h, region.x, region.y
                            );
                            match run_ocr_pipeline(
                                &region,
                                &ocr_wrapper,
                                &cfg.ocr.language,
                                cfg.ocr.cleanup,
                                delivery_mode,
                                &cfg.tts.voice,
                                cfg.tts.speed,
                                &mut tts_state,
                                &venv_python,
                                &daemon_script,
                                &models_dir,
                                &tts_wrapper,
                            ) {
                                Ok(n) => {
                                    gui_send!(gui_stdin, "ocr:{n} chars");
                                    gui_send!(gui_stdin, "tts:speaking");
                                }
                                Err(e) => eprintln!("[deck-reader] Pipeline error: {e}"),
                            }
                        }
                        Err(e) => eprintln!("[deck-reader] {e}"),
                    }

                    while let Ok(ev) = key_rx.try_recv() {
                        match ev {
                            EventType::KeyPress(k)   => { held.insert(k); }
                            EventType::KeyRelease(k) => { held.remove(&k); }
                            _ => {}
                        }
                    }
                    busy = false;
                    gui_send!(gui_stdin, "status:Listening...");
                    println!("[deck-reader] Ready.");
                }
                // ── Alt+Y: TTS toggle (always available) ────────────────
                else if hotkey_matches(k, hk_tts, &held) {
                    if tts_state.speaking {
                        // Already speaking — stop.
                        stop_tts(&mut tts_state);
                        gui_send!(gui_stdin, "tts:idle");
                        println!("[deck-reader] TTS stopped.");
                    } else {
                        // Idle — read selection and speak.
                        match platform::read_clipboard() {
                            Ok(text) if !text.is_empty() => {
                                gui_send!(gui_stdin, "tts:speaking");
                                if let Err(e) = request_tts(
                                    &text,
                                    &cfg.tts.voice,
                                    cfg.tts.speed,
                                    &mut tts_state,
                                    &venv_python,
                                    &daemon_script,
                                    &models_dir,
                                    &tts_wrapper,
                                ) {
                                    gui_send!(gui_stdin, "tts:error");
                                    eprintln!("[deck-reader] TTS error: {e}");
                                }
                            }
                            Ok(_) => println!(
                                "[deck-reader] Selection/clipboard is empty — \
                                nothing to speak.\n\
                                Tip: highlight text first; Electron apps may need \
                                Ctrl+C before Alt+Y."
                            ),
                            Err(e) => eprintln!("[deck-reader] Clipboard error: {e}"),
                        }
                    }
                }
            }

            _ => {}
        }
    }

    // ── Cleanup: stop TTS, shut down daemon, close GUI ─────────────────────
    shutdown_daemon(&mut tts_state);
    drop(gui_stdin); // close stdin pipe → GUI's reader thread detects EOF → window closes
    if let Some(mut child) = gui_child {
        let _ = child.wait();
    }
    println!("[deck-reader] Exited.");

    Ok(())
}
