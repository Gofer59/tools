use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub struct HotkeyHandle {
    pub current: String,
    pub action: String,
    pub fallback_thread: Option<std::thread::JoinHandle<()>>,
    pub fallback_stop: Arc<AtomicBool>,
    pub using_fallback: bool,
}

/// Register a hotkey. Emits "hotkey-triggered" with payload:
///   `{"tool":"screen-ocr","action":"<action>","state":"pressed"|"released"}`
pub fn register(app: &AppHandle, key: &str, action: &str) -> Result<HotkeyHandle> {
    let stop = Arc::new(AtomicBool::new(false));

    let app2 = app.clone();
    let action_owned = action.to_string();
    let key_owned = key.to_string();
    let action_for_gs = action_owned.clone();

    let gs_result = app.global_shortcut().on_shortcut(key, move |_app, _shortcut, ev| {
        let state = if ev.state() == ShortcutState::Pressed {
            "pressed"
        } else {
            "released"
        };
        let _ = app2.emit(
            "hotkey-triggered",
            serde_json::json!({
                "tool": "screen-ocr",
                "action": action_for_gs,
                "state": state,
            }),
        );
    });

    if gs_result.is_ok() {
        return Ok(HotkeyHandle {
            current: key_owned,
            action: action_owned,
            fallback_thread: None,
            fallback_stop: stop,
            using_fallback: false,
        });
    }

    // Fallback: rdev evdev listen thread (Linux Wayland / game-window scenarios)
    let stop2 = stop.clone();
    let app3 = app.clone();
    let key_for_thread = key_owned.clone();
    let action_for_thread = action_owned.clone();

    let fallback_thread = Some(std::thread::spawn(move || {
        rdev_fallback_loop(app3, key_for_thread, action_for_thread, stop2);
    }));

    Ok(HotkeyHandle {
        current: key_owned,
        action: action_owned,
        fallback_thread,
        fallback_stop: stop,
        using_fallback: true,
    })
}

pub fn unregister(app: &AppHandle, h: &mut HotkeyHandle) {
    if !h.using_fallback {
        let _ = app.global_shortcut().unregister(h.current.as_str());
    }
    h.fallback_stop.store(true, Ordering::SeqCst);
    if let Some(t) = h.fallback_thread.take() {
        // rdev::listen is blocking; signal stop and detach — thread exits on next event
        drop(t);
    }
}

/// Parse an accelerator string like "Ctrl+Alt+Space" into rdev key names.
fn parse_accelerator(acc: &str) -> Vec<String> {
    acc.split('+').map(|s| s.trim().to_lowercase()).collect()
}

fn rdev_key_name(key: &rdev::Key) -> &'static str {
    match key {
        rdev::Key::ControlLeft | rdev::Key::ControlRight => "ctrl",
        rdev::Key::Alt => "alt",
        rdev::Key::AltGr => "altgr",
        rdev::Key::ShiftLeft | rdev::Key::ShiftRight => "shift",
        rdev::Key::MetaLeft | rdev::Key::MetaRight => "meta",
        rdev::Key::Space => "space",
        rdev::Key::Return => "return",
        rdev::Key::Escape => "escape",
        rdev::Key::Tab => "tab",
        rdev::Key::Backspace => "backspace",
        rdev::Key::Delete => "delete",
        rdev::Key::F1 => "f1",
        rdev::Key::F2 => "f2",
        rdev::Key::F3 => "f3",
        rdev::Key::F4 => "f4",
        rdev::Key::F5 => "f5",
        rdev::Key::F6 => "f6",
        rdev::Key::F7 => "f7",
        rdev::Key::F8 => "f8",
        rdev::Key::F9 => "f9",
        rdev::Key::F10 => "f10",
        rdev::Key::F11 => "f11",
        rdev::Key::F12 => "f12",
        _ => "unknown",
    }
}

fn rdev_fallback_loop(
    app: AppHandle,
    accelerator: String,
    action: String,
    stop: Arc<AtomicBool>,
) {
    let target_keys = parse_accelerator(&accelerator);
    let held: Arc<std::sync::Mutex<std::collections::HashSet<String>>> =
        Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
    let was_triggered = Arc::new(AtomicBool::new(false));

    let held2 = held.clone();
    let was_triggered2 = was_triggered.clone();
    let stop2 = stop.clone();
    let app2 = app.clone();
    let target2 = target_keys.clone();
    let action2 = action.clone();

    let _ = rdev::listen(move |ev| {
        if stop2.load(Ordering::SeqCst) {
            return;
        }
        match ev.event_type {
            rdev::EventType::KeyPress(key) => {
                let name = rdev_key_name(&key).to_string();
                let mut h = held2.lock().unwrap();
                h.insert(name);
                let all_held = target2.iter().all(|k| h.contains(k));
                if all_held && !was_triggered2.load(Ordering::SeqCst) {
                    was_triggered2.store(true, Ordering::SeqCst);
                    let _ = app2.emit(
                        "hotkey-triggered",
                        serde_json::json!({
                            "tool": "screen-ocr",
                            "action": action2,
                            "state": "pressed",
                        }),
                    );
                }
            }
            rdev::EventType::KeyRelease(key) => {
                let name = rdev_key_name(&key).to_string();
                let mut h = held2.lock().unwrap();
                h.remove(&name);
                if was_triggered2.load(Ordering::SeqCst) {
                    let all_held = target2.iter().all(|k| h.contains(k));
                    if !all_held {
                        was_triggered2.store(false, Ordering::SeqCst);
                        let _ = app2.emit(
                            "hotkey-triggered",
                            serde_json::json!({
                                "tool": "screen-ocr",
                                "action": action2,
                                "state": "released",
                            }),
                        );
                    }
                }
            }
            _ => {}
        }
    });
}
