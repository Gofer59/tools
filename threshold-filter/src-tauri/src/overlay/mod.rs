pub mod capture;
pub mod hotkey;
pub mod processing;
pub mod ui;

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Config structs — JSON format, stored at the data_local_dir path
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(default)]
pub struct OverlayConfig {
    pub region_select_hotkey: String,
    pub toggle_on_top_hotkey: String,
    pub default_threshold: u8,
    pub default_invert: bool,
    pub default_always_on_top: bool,
    #[serde(default)]
    pub auto_start_overlay: bool,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            region_select_hotkey: "F10".to_string(),
            toggle_on_top_hotkey: "F9".to_string(),
            default_threshold: 128,
            default_invert: false,
            default_always_on_top: true,
            auto_start_overlay: false,
        }
    }
}

fn config_path() -> Option<std::path::PathBuf> {
    dirs::data_local_dir().map(|d| d.join("threshold-filter").join("config.json"))
}

fn load_config() -> OverlayConfig {
    let Some(path) = config_path() else {
        return OverlayConfig::default();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return OverlayConfig::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Hotkey string → rdev key pair
// ---------------------------------------------------------------------------

fn parse_hotkey(s: &str) -> anyhow::Result<(Option<rdev::Key>, rdev::Key)> {
    match s.splitn(2, '+').collect::<Vec<_>>().as_slice() {
        [single] => Ok((None, str_to_rdev_key(single.trim())?)),
        [modifier, action] => Ok((
            Some(str_to_rdev_key(modifier.trim())?),
            str_to_rdev_key(action.trim())?,
        )),
        _ => anyhow::bail!("Invalid hotkey string: {s:?}"),
    }
}

fn str_to_rdev_key(s: &str) -> anyhow::Result<rdev::Key> {
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
        "Backspace"                 => Ok(Key::Backspace),
        "CapsLock"                  => Ok(Key::CapsLock),
        "Delete"                    => Ok(Key::Delete),
        "DownArrow" | "Down"        => Ok(Key::DownArrow),
        "End"                       => Ok(Key::End),
        "Escape" | "Esc"            => Ok(Key::Escape),
        "Home"                      => Ok(Key::Home),
        "LeftArrow" | "Left"        => Ok(Key::LeftArrow),
        "Return" | "Enter"          => Ok(Key::Return),
        "RightArrow" | "Right"      => Ok(Key::RightArrow),
        "Space"                     => Ok(Key::Space),
        "Tab"                       => Ok(Key::Tab),
        "UpArrow" | "Up"            => Ok(Key::UpArrow),
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
            anyhow::bail!("Unknown key name {other:?}. Use F9, MetaLeft+KeyQ, 191, etc.");
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run_overlay() {
    env_logger::init();

    let cfg = load_config();

    let hk_reselect = parse_hotkey(&cfg.region_select_hotkey)
        .unwrap_or_else(|e| {
            eprintln!("[threshold-filter] Bad region_select_hotkey: {e}; defaulting to F10");
            (None, rdev::Key::F10)
        });
    let hk_toggle_top = parse_hotkey(&cfg.toggle_on_top_hotkey)
        .unwrap_or_else(|e| {
            eprintln!("[threshold-filter] Bad toggle_on_top_hotkey: {e}; defaulting to F9");
            (None, rdev::Key::F9)
        });

    let always_on_top = cfg.default_always_on_top;
    let default_threshold = cfg.default_threshold;
    let invert = cfg.default_invert;
    let key_reselect_name = cfg.region_select_hotkey.clone();
    let key_toggle_top_name = cfg.toggle_on_top_hotkey.clone();

    let (action_tx, action_rx) = std::sync::mpsc::channel::<hotkey::HotkeyAction>();
    hotkey::spawn_hotkey_listener(action_tx, hk_reselect, hk_toggle_top);

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Threshold Filter")
        .with_inner_size([640.0, 480.0])
        .with_min_inner_size([200.0, 150.0])
        .with_decorations(false);
    if always_on_top {
        viewport = viewport.with_always_on_top();
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "threshold-filter",
        options,
        Box::new(move |cc| {
            Ok(Box::new(ui::ThresholdApp::new(
                cc,
                action_rx,
                key_reselect_name,
                key_toggle_top_name,
                default_threshold,
                invert,
                always_on_top,
            )))
        }),
    )
    .unwrap_or_else(|e| eprintln!("[threshold-filter] eframe error: {e}"));
}
