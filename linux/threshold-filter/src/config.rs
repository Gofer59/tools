use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

const DEFAULT_CONFIG_TOML: &str = r#"[hotkeys]
# Key names: F1-F12, Escape, Tab, Space, A-Z, etc.
# Modifier combos: MetaLeft+KeyQ, AltLeft+KeyU, ControlLeft+KeyR
# Raw keycodes: "191" or "Unknown(191)"
region_select   = "F10"
toggle_on_top   = "F9"

[display]
default_threshold = 128       # 0-255
invert            = false     # swap black/white
always_on_top     = true
"#;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    pub hotkeys: HotkeyConfig,
    pub display: DisplayConfig,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HotkeyConfig {
    pub region_select: String,
    pub toggle_on_top: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisplayConfig {
    #[serde(default = "default_threshold")]
    pub default_threshold: u8,
    #[serde(default)]
    pub invert: bool,
    #[serde(default = "default_true")]
    pub always_on_top: bool,
}

fn default_threshold() -> u8 {
    128
}
fn default_true() -> bool {
    true
}

pub fn config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("threshold-filter");
        }
    }
    let base = std::env::var("XDG_CONFIG_HOME")
        .unwrap_or_else(|_| format!("{}/.config", std::env::var("HOME").unwrap_or_default()));
    PathBuf::from(base).join("threshold-filter")
}

pub fn load_config() -> Result<AppConfig> {
    let dir = config_dir();
    let path = dir.join("config.toml");

    if !path.exists() {
        fs::create_dir_all(&dir)
            .with_context(|| format!("Cannot create config directory {dir:?}"))?;
        fs::write(&path, DEFAULT_CONFIG_TOML)
            .with_context(|| format!("Cannot write default config to {path:?}"))?;
        log::info!("Created default config at {path:?}");
    }

    let text = fs::read_to_string(&path)
        .with_context(|| format!("Cannot read config file {path:?}"))?;

    toml::from_str(&text)
        .with_context(|| format!("Malformed config at {path:?} -- fix the TOML then restart"))
}

// ---------------------------------------------------------------------------
// rdev key parsing — same pattern as steamdeck/src/main.rs
// ---------------------------------------------------------------------------

fn str_to_rdev_key(s: &str) -> Result<rdev::Key> {
    use rdev::Key;
    match s.trim() {
        // Modifiers
        "MetaLeft" => Ok(Key::MetaLeft),
        "MetaRight" => Ok(Key::MetaRight),
        "AltLeft" | "Alt" => Ok(Key::Alt),
        "AltRight" | "AltGr" => Ok(Key::AltGr),
        "ShiftLeft" => Ok(Key::ShiftLeft),
        "ShiftRight" => Ok(Key::ShiftRight),
        "ControlLeft" | "Control" | "Ctrl" => Ok(Key::ControlLeft),
        "ControlRight" => Ok(Key::ControlRight),
        // Letters (accept both "A" and "KeyA" style)
        "A" | "KeyA" => Ok(Key::KeyA), "B" | "KeyB" => Ok(Key::KeyB),
        "C" | "KeyC" => Ok(Key::KeyC), "D" | "KeyD" => Ok(Key::KeyD),
        "E" | "KeyE" => Ok(Key::KeyE), "F" | "KeyF" => Ok(Key::KeyF),
        "G" | "KeyG" => Ok(Key::KeyG), "H" | "KeyH" => Ok(Key::KeyH),
        "I" | "KeyI" => Ok(Key::KeyI), "J" | "KeyJ" => Ok(Key::KeyJ),
        "K" | "KeyK" => Ok(Key::KeyK), "L" | "KeyL" => Ok(Key::KeyL),
        "M" | "KeyM" => Ok(Key::KeyM), "N" | "KeyN" => Ok(Key::KeyN),
        "O" | "KeyO" => Ok(Key::KeyO), "P" | "KeyP" => Ok(Key::KeyP),
        "Q" | "KeyQ" => Ok(Key::KeyQ), "R" | "KeyR" => Ok(Key::KeyR),
        "S" | "KeyS" => Ok(Key::KeyS), "T" | "KeyT" => Ok(Key::KeyT),
        "U" | "KeyU" => Ok(Key::KeyU), "V" | "KeyV" => Ok(Key::KeyV),
        "W" | "KeyW" => Ok(Key::KeyW), "X" | "KeyX" => Ok(Key::KeyX),
        "Y" | "KeyY" => Ok(Key::KeyY), "Z" | "KeyZ" => Ok(Key::KeyZ),
        // F-keys
        "F1" => Ok(Key::F1), "F2" => Ok(Key::F2), "F3" => Ok(Key::F3),
        "F4" => Ok(Key::F4), "F5" => Ok(Key::F5), "F6" => Ok(Key::F6),
        "F7" => Ok(Key::F7), "F8" => Ok(Key::F8), "F9" => Ok(Key::F9),
        "F10" => Ok(Key::F10), "F11" => Ok(Key::F11), "F12" => Ok(Key::F12),
        // Common keys
        "Backspace" => Ok(Key::Backspace),
        "CapsLock" => Ok(Key::CapsLock),
        "Delete" => Ok(Key::Delete),
        "DownArrow" | "Down" => Ok(Key::DownArrow),
        "End" => Ok(Key::End),
        "Escape" | "Esc" => Ok(Key::Escape),
        "Home" => Ok(Key::Home),
        "LeftArrow" | "Left" => Ok(Key::LeftArrow),
        "Return" | "Enter" => Ok(Key::Return),
        "RightArrow" | "Right" => Ok(Key::RightArrow),
        "Space" => Ok(Key::Space),
        "Tab" => Ok(Key::Tab),
        "UpArrow" | "Up" => Ok(Key::UpArrow),
        // Raw keycodes: "Unknown(191)" or just "191"
        other => {
            if let Some(inner) = other
                .strip_prefix("Unknown(")
                .and_then(|s| s.strip_suffix(')'))
            {
                let code: u32 = inner.trim().parse()
                    .with_context(|| format!("Invalid raw keycode in {other:?}"))?;
                return Ok(Key::Unknown(code));
            }
            if let Ok(code) = other.parse::<u32>() {
                return Ok(Key::Unknown(code));
            }
            anyhow::bail!("Unknown key name {other:?}. Use F9, MetaLeft+KeyQ, 191, etc.");
        }
    }
}

/// Parse "F10" or "MetaLeft+KeyQ" into (optional modifier, action key).
pub fn parse_hotkey(s: &str) -> Result<(Option<rdev::Key>, rdev::Key)> {
    match s.splitn(2, '+').collect::<Vec<_>>().as_slice() {
        [single] => Ok((None, str_to_rdev_key(single.trim())?)),
        [modifier, action] => Ok((Some(str_to_rdev_key(modifier.trim())?), str_to_rdev_key(action.trim())?)),
        _ => anyhow::bail!("Invalid hotkey string: {s:?}"),
    }
}
