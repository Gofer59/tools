mod capture;
mod config;
mod processing;
mod ui;

use std::collections::HashSet;
use std::sync::mpsc;

use anyhow::{Context, Result};
use rdev::{listen, Event, EventType, Key};

use crate::capture::{DisplayServer, SelectionResult};
use crate::config::{data_dir, load_config};
use crate::ui::{HotkeyAction, ThresholdApp};

// ---------------------------------------------------------------------------
// Hotkey parsing — same pattern as deck-reader
// ---------------------------------------------------------------------------

fn str_to_key(s: &str) -> Result<Key> {
    match s.trim() {
        "MetaLeft" => Ok(Key::MetaLeft),
        "MetaRight" => Ok(Key::MetaRight),
        "AltLeft" | "Alt" => Ok(Key::Alt),
        "AltRight" | "AltGr" => Ok(Key::AltGr),
        "ShiftLeft" => Ok(Key::ShiftLeft),
        "ShiftRight" => Ok(Key::ShiftRight),
        "ControlLeft" | "Control" | "Ctrl" => Ok(Key::ControlLeft),
        "ControlRight" => Ok(Key::ControlRight),
        "KeyA" => Ok(Key::KeyA), "KeyB" => Ok(Key::KeyB), "KeyC" => Ok(Key::KeyC),
        "KeyD" => Ok(Key::KeyD), "KeyE" => Ok(Key::KeyE), "KeyF" => Ok(Key::KeyF),
        "KeyG" => Ok(Key::KeyG), "KeyH" => Ok(Key::KeyH), "KeyI" => Ok(Key::KeyI),
        "KeyJ" => Ok(Key::KeyJ), "KeyK" => Ok(Key::KeyK), "KeyL" => Ok(Key::KeyL),
        "KeyM" => Ok(Key::KeyM), "KeyN" => Ok(Key::KeyN), "KeyO" => Ok(Key::KeyO),
        "KeyP" => Ok(Key::KeyP), "KeyQ" => Ok(Key::KeyQ), "KeyR" => Ok(Key::KeyR),
        "KeyS" => Ok(Key::KeyS), "KeyT" => Ok(Key::KeyT), "KeyU" => Ok(Key::KeyU),
        "KeyV" => Ok(Key::KeyV), "KeyW" => Ok(Key::KeyW), "KeyX" => Ok(Key::KeyX),
        "KeyY" => Ok(Key::KeyY), "KeyZ" => Ok(Key::KeyZ),
        "F1" => Ok(Key::F1), "F2" => Ok(Key::F2), "F3" => Ok(Key::F3),
        "F4" => Ok(Key::F4), "F5" => Ok(Key::F5), "F6" => Ok(Key::F6),
        "F7" => Ok(Key::F7), "F8" => Ok(Key::F8), "F9" => Ok(Key::F9),
        "F10" => Ok(Key::F10), "F11" => Ok(Key::F11), "F12" => Ok(Key::F12),
        "Backspace" => Ok(Key::Backspace),
        "CapsLock" => Ok(Key::CapsLock),
        "Delete" => Ok(Key::Delete),
        "DownArrow" => Ok(Key::DownArrow),
        "End" => Ok(Key::End),
        "Escape" | "Esc" => Ok(Key::Escape),
        "Home" => Ok(Key::Home),
        "LeftArrow" => Ok(Key::LeftArrow),
        "Return" | "Enter" => Ok(Key::Return),
        "RightArrow" => Ok(Key::RightArrow),
        "Space" => Ok(Key::Space),
        "Tab" => Ok(Key::Tab),
        "UpArrow" => Ok(Key::UpArrow),
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

fn parse_hotkey(s: &str) -> Result<(Option<Key>, Key)> {
    match s.splitn(2, '+').collect::<Vec<_>>().as_slice() {
        [single] => Ok((None, str_to_key(single.trim())?)),
        [modifier, action] => Ok((Some(str_to_key(modifier.trim())?), str_to_key(action.trim())?)),
        _ => anyhow::bail!("Invalid hotkey string: {s:?}"),
    }
}

fn hotkey_matches(k: Key, hk: (Option<Key>, Key), held: &HashSet<Key>) -> bool {
    k == hk.1 && hk.0.is_none_or(|m| held.contains(&m))
}

// ---------------------------------------------------------------------------
// Hotkey dispatcher — translates raw rdev events into app actions
// ---------------------------------------------------------------------------

fn spawn_hotkey_dispatcher(
    action_tx: mpsc::Sender<HotkeyAction>,
    region_tx: mpsc::Sender<Result<SelectionResult, String>>,
    region_file: std::path::PathBuf,
    display: DisplayServer,
    hk_select: (Option<Key>, Key),
    hk_toggle_top: (Option<Key>, Key),
) {
    // Raw rdev listener thread
    let (raw_tx, raw_rx) = mpsc::channel::<EventType>();
    std::thread::spawn(move || {
        if let Err(e) = listen(move |event: Event| {
            if matches!(
                &event.event_type,
                EventType::KeyPress(_) | EventType::KeyRelease(_)
            ) {
                let _ = raw_tx.send(event.event_type);
            }
        }) {
            eprintln!("[threshold-filter] rdev listener error: {e:?}");
            eprintln!("[threshold-filter] Is the current user in the 'input' group?");
        }
    });

    // Dispatcher thread
    std::thread::spawn(move || {
        let mut held: HashSet<Key> = HashSet::new();

        loop {
            let event = match raw_rx.recv_timeout(std::time::Duration::from_millis(200)) {
                Ok(e) => e,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => break,
            };

            match event {
                EventType::KeyRelease(k) => {
                    held.remove(&k);
                }
                EventType::KeyPress(k) => {
                    held.insert(k);

                    if hotkey_matches(k, hk_select, &held) {
                        eprintln!("[threshold-filter] Region select hotkey matched...");
                        // Block dispatcher on selection tool — same pattern as deck-reader
                        match capture::select_region(display) {
                            Ok(result) => {
                                if let Err(e) = capture::save_region(&region_file, &result.screen_region) {
                                    eprintln!("[threshold-filter] Save region failed: {e}");
                                }
                                let _ = region_tx.send(Ok(result));
                            }
                            Err(e) => {
                                eprintln!("[threshold-filter] Region select failed: {e}");
                                let _ = region_tx.send(Err(format!("{e}")));
                            }
                        }
                        // Drain queued events and reset held keys
                        while raw_rx.try_recv().is_ok() {}
                        held.clear();
                    }

                    if hotkey_matches(k, hk_toggle_top, &held) {
                        let _ = action_tx.send(HotkeyAction::ToggleOnTop);
                    }
                }
                _ => {}
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    env_logger::init();

    let cfg = load_config()?;

    let hk_select = parse_hotkey(&cfg.hotkeys.region_select)?;
    let hk_toggle_top = parse_hotkey(&cfg.hotkeys.toggle_on_top)?;

    let region_file = data_dir().join("last_region.json");

    // Detect display server
    let display = capture::detect_display_server();
    eprintln!("[threshold-filter] Display server: {display:?}");

    // Channels
    let (action_tx, action_rx) = mpsc::channel::<HotkeyAction>();
    let (region_tx, region_rx) = mpsc::channel::<Result<SelectionResult, String>>();
    let region_tx_ui = region_tx.clone();

    // Spawn hotkey dispatcher
    spawn_hotkey_dispatcher(action_tx, region_tx, region_file.clone(), display, hk_select, hk_toggle_top);

    // Launch eframe
    let default_threshold = cfg.display.default_threshold;
    let invert = cfg.display.invert;
    let always_on_top = cfg.display.always_on_top;
    let panel_width = cfg.display.panel_width;

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Threshold Filter (Deck)")
        .with_inner_size([800.0, 600.0])
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
        "threshold-filter-deck",
        options,
        Box::new(move |cc| {
            Ok(Box::new(ThresholdApp::new(
                cc,
                action_rx,
                region_rx,
                region_tx_ui,
                region_file,
                display,
                default_threshold,
                invert,
                always_on_top,
                panel_width,
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))
}
