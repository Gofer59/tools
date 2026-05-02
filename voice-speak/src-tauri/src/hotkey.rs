use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub struct HotkeyHandle {
    pub current: String,
    pub fallback_thread: Option<std::thread::JoinHandle<()>>,
    pub fallback_stop: Arc<AtomicBool>,
    pub using_fallback: bool,
}

pub fn register(app: &AppHandle, accelerator: &str) -> Result<HotkeyHandle> {
    let stop = Arc::new(AtomicBool::new(false));

    let app2 = app.clone();
    let acc_owned = accelerator.to_string();

    let gs_result = app.global_shortcut().on_shortcut(accelerator, move |_app, _shortcut, ev| {
        let state = if ev.state() == ShortcutState::Pressed {
            "pressed"
        } else {
            "released"
        };
        let _ = app2.emit(
            "hotkey-triggered",
            serde_json::json!({"tool": "voice-speak", "state": state}),
        );
    });

    if gs_result.is_ok() {
        return Ok(HotkeyHandle {
            current: acc_owned,
            fallback_thread: None,
            fallback_stop: stop,
            using_fallback: false,
        });
    }

    // Fallback: rdev evdev listen thread (Linux Wayland / game-window scenarios)
    let stop2 = stop.clone();
    let app3 = app.clone();
    let acc_for_thread = acc_owned.clone();

    let fallback_thread = Some(std::thread::spawn(move || {
        rdev_fallback_loop(app3, acc_for_thread, stop2);
    }));

    Ok(HotkeyHandle {
        current: acc_owned,
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
        // rdev::listen is blocking; we signal stop and detach — thread exits on next event
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

/// Spawn a one-shot rdev listener that captures the next key combination and
/// emits `hotkey-triggered` with `{captured: "Mod+Mod+Key"}`.
pub fn spawn_capture_thread(app: AppHandle, armed: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        use rdev::{listen, EventType};
        use std::collections::HashSet;
        use std::sync::Mutex;

        let held: Arc<Mutex<HashSet<rdev::Key>>> = Arc::new(Mutex::new(HashSet::new()));
        let done = Arc::new(AtomicBool::new(false));

        let held2 = held.clone();
        let done2 = done.clone();
        let armed2 = armed.clone();
        let app2 = app.clone();

        let _ = listen(move |ev| {
            if done2.load(Ordering::SeqCst) || !armed2.load(Ordering::SeqCst) {
                return;
            }
            match ev.event_type {
                EventType::KeyPress(key) => {
                    held2.lock().unwrap().insert(key);
                }
                EventType::KeyRelease(_key) => {
                    let snapshot = held2.lock().unwrap().clone();
                    let combo = format_key_combo(&snapshot);
                    if combo.is_empty() { return; }
                    done2.store(true, Ordering::SeqCst);
                    armed2.store(false, Ordering::SeqCst);
                    let _ = app2.emit(
                        "hotkey-triggered",
                        serde_json::json!({
                            "tool": "voice-speak",
                            "state": "captured",
                            "captured": combo,
                        }),
                    );
                }
                _ => {}
            }
        });
    });
}

fn format_key_combo(held: &std::collections::HashSet<rdev::Key>) -> String {
    use rdev::Key;
    let mut has_ctrl = false; let mut has_alt = false;
    let mut has_shift = false; let mut has_super = false;
    let mut main: Option<&'static str> = None;
    for key in held {
        match key {
            Key::ControlLeft | Key::ControlRight => has_ctrl = true,
            Key::Alt | Key::AltGr              => has_alt = true,
            Key::ShiftLeft | Key::ShiftRight   => has_shift = true,
            Key::MetaLeft | Key::MetaRight     => has_super = true,
            _ => { if main.is_none() { main = rdev_key_to_accel(key); } }
        }
    }
    let key_str = match main { Some(k) => k, None => return String::new() };
    let mut parts: Vec<&str> = Vec::new();
    if has_ctrl  { parts.push("Ctrl"); }
    if has_alt   { parts.push("Alt"); }
    if has_shift { parts.push("Shift"); }
    if has_super { parts.push("Super"); }
    parts.push(key_str);
    parts.join("+")
}

fn rdev_key_to_accel(key: &rdev::Key) -> Option<&'static str> {
    use rdev::Key;
    Some(match key {
        Key::Space => "Space", Key::Return => "Return", Key::Escape => "Escape",
        Key::Tab => "Tab", Key::Backspace => "Backspace", Key::Delete => "Delete",
        Key::F1 => "F1", Key::F2 => "F2", Key::F3 => "F3", Key::F4 => "F4",
        Key::F5 => "F5", Key::F6 => "F6", Key::F7 => "F7", Key::F8 => "F8",
        Key::F9 => "F9", Key::F10 => "F10", Key::F11 => "F11", Key::F12 => "F12",
        Key::UpArrow => "Up", Key::DownArrow => "Down",
        Key::LeftArrow => "Left", Key::RightArrow => "Right",
        Key::Home => "Home", Key::End => "End",
        Key::PageUp => "PageUp", Key::PageDown => "PageDown",
        Key::KeyA => "A", Key::KeyB => "B", Key::KeyC => "C", Key::KeyD => "D",
        Key::KeyE => "E", Key::KeyF => "F", Key::KeyG => "G", Key::KeyH => "H",
        Key::KeyI => "I", Key::KeyJ => "J", Key::KeyK => "K", Key::KeyL => "L",
        Key::KeyM => "M", Key::KeyN => "N", Key::KeyO => "O", Key::KeyP => "P",
        Key::KeyQ => "Q", Key::KeyR => "R", Key::KeyS => "S", Key::KeyT => "T",
        Key::KeyU => "U", Key::KeyV => "V", Key::KeyW => "W", Key::KeyX => "X",
        Key::KeyY => "Y", Key::KeyZ => "Z",
        Key::Num0 => "Num0", Key::Num1 => "Num1", Key::Num2 => "Num2",
        Key::Num3 => "Num3", Key::Num4 => "Num4", Key::Num5 => "Num5",
        Key::Num6 => "Num6", Key::Num7 => "Num7", Key::Num8 => "Num8",
        Key::Num9 => "Num9",
        _ => return None,
    })
}

fn rdev_fallback_loop(app: AppHandle, accelerator: String, stop: Arc<AtomicBool>) {
    let target_keys = parse_accelerator(&accelerator);
    let held: Arc<std::sync::Mutex<std::collections::HashSet<String>>> =
        Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
    let was_triggered = Arc::new(AtomicBool::new(false));

    let held2 = held.clone();
    let was_triggered2 = was_triggered.clone();
    let stop2 = stop.clone();
    let app2 = app.clone();
    let target2 = target_keys.clone();

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
                        serde_json::json!({"tool": "voice-speak", "state": "pressed"}),
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
                            serde_json::json!({"tool": "voice-speak", "state": "released"}),
                        );
                    }
                }
            }
            _ => {}
        }
    });
}
