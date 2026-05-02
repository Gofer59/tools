use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub region_select_hotkey: String,
    pub toggle_on_top_hotkey: String,
    pub default_threshold: u8,
    pub default_invert: bool,
    pub default_always_on_top: bool,
    pub auto_start_overlay: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            region_select_hotkey: "F10".to_string(),
            toggle_on_top_hotkey: "F9".to_string(),
            default_threshold: 128,
            default_invert: false,
            default_always_on_top: true,
            auto_start_overlay: true,
        }
    }
}

fn config_path(dir: &Path) -> PathBuf {
    dir.join("config.json")
}

pub fn load_or_default(dir: &Path) -> Config {
    let p = config_path(dir);
    if let Ok(bytes) = std::fs::read(&p) {
        if let Ok(cfg) = serde_json::from_slice::<Config>(&bytes) {
            return cfg;
        }
    }
    Config::default()
}

pub fn save(dir: &Path, cfg: &Config) -> Result<()> {
    let p = config_path(dir);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(p, serde_json::to_vec_pretty(cfg)?)?;
    Ok(())
}

/// Parse a hotkey string like "F10" or "Alt+KeyA" into an (optional modifier, action key) pair.
///
/// Examples:
///   "F10"      -> (None, Key::F10)
///   "Alt+KeyA" -> (Some(Key::Alt), Key::KeyA)
pub fn parse_hotkey(s: &str) -> Result<(Option<rdev::Key>, rdev::Key)> {
    match s.splitn(2, '+').collect::<Vec<_>>().as_slice() {
        [single] => Ok((None, str_to_rdev_key(single.trim())?)),
        [modifier, action] => Ok((
            Some(str_to_rdev_key(modifier.trim())?),
            str_to_rdev_key(action.trim())?,
        )),
        _ => anyhow::bail!("Invalid hotkey string: {s:?}"),
    }
}

fn str_to_rdev_key(s: &str) -> Result<rdev::Key> {
    use anyhow::Context;
    use rdev::Key;
    match s.trim() {
        "MetaLeft"                                   => Ok(Key::MetaLeft),
        "MetaRight"                                  => Ok(Key::MetaRight),
        "AltLeft" | "Alt"                            => Ok(Key::Alt),
        "AltRight" | "AltGr"                         => Ok(Key::AltGr),
        "ShiftLeft"                                  => Ok(Key::ShiftLeft),
        "ShiftRight"                                 => Ok(Key::ShiftRight),
        "ControlLeft" | "Control" | "Ctrl"           => Ok(Key::ControlLeft),
        "ControlRight"                               => Ok(Key::ControlRight),
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
        "F1"  => Ok(Key::F1),  "F2"  => Ok(Key::F2),  "F3"  => Ok(Key::F3),
        "F4"  => Ok(Key::F4),  "F5"  => Ok(Key::F5),  "F6"  => Ok(Key::F6),
        "F7"  => Ok(Key::F7),  "F8"  => Ok(Key::F8),  "F9"  => Ok(Key::F9),
        "F10" => Ok(Key::F10), "F11" => Ok(Key::F11), "F12" => Ok(Key::F12),
        "Backspace"             => Ok(Key::Backspace),
        "CapsLock"              => Ok(Key::CapsLock),
        "Delete"                => Ok(Key::Delete),
        "DownArrow" | "Down"    => Ok(Key::DownArrow),
        "End"                   => Ok(Key::End),
        "Escape" | "Esc"        => Ok(Key::Escape),
        "Home"                  => Ok(Key::Home),
        "LeftArrow" | "Left"    => Ok(Key::LeftArrow),
        "Return" | "Enter"      => Ok(Key::Return),
        "RightArrow" | "Right"  => Ok(Key::RightArrow),
        "Space"                 => Ok(Key::Space),
        "Tab"                   => Ok(Key::Tab),
        "UpArrow" | "Up"        => Ok(Key::UpArrow),
        other => {
            if let Some(inner) = other
                .strip_prefix("Unknown(")
                .and_then(|s| s.strip_suffix(')'))
            {
                let code: u32 = inner
                    .trim()
                    .parse()
                    .with_context(|| format!("Invalid raw keycode in {other:?}"))?;
                return Ok(Key::Unknown(code));
            }
            if let Ok(code) = other.parse::<u32>() {
                return Ok(Key::Unknown(code));
            }
            anyhow::bail!("Unknown key name {other:?}. Use F9, Alt+KeyQ, 191, etc.");
        }
    }
}
