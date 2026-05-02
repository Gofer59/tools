use std::sync::{Arc, atomic::Ordering};

use tauri::{AppHandle, Emitter, State};

use crate::AppState;

/// Arm a one-shot rdev listener to capture the next keypress and report it back
/// via the "hotkey-captured" event.
///
/// `which` is an opaque tag the UI passes to correlate the result (e.g.
/// "region_select_hotkey" or "toggle_on_top_hotkey").
#[tauri::command]
pub async fn test_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
    which: String,
) -> Result<(), String> {
    // Prevent concurrent capture sessions.
    if state.test_hotkey_armed.swap(true, Ordering::SeqCst) {
        return Err("hotkey capture already in progress".to_string());
    }
    let _ = app.emit("hotkey-test-armed", serde_json::json!({"which": which}));

    let armed2 = state.test_hotkey_armed.clone();

    std::thread::spawn(move || {
        // Collect held keys so we can build "Modifier+Key" strings.
        let held: Arc<std::sync::Mutex<std::collections::HashSet<rdev::Key>>> =
            Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
        let held2 = held.clone();

        let _ = rdev::listen(move |ev| {
            if !armed2.load(Ordering::SeqCst) {
                return;
            }
            match ev.event_type {
                rdev::EventType::KeyPress(key) => {
                    let mut h = held2.lock().unwrap();
                    h.insert(key);

                    // Only capture non-modifier keys as the action key.
                    if !is_modifier(key) {
                        let modifier = h
                            .iter()
                            .find(|&&k| is_modifier(k))
                            .copied();

                        let key_str = rdev_key_to_str(key);
                        // Skip unrecognized keys — they'd produce an unparseable string.
                        if key_str.is_none() {
                            return;
                        }
                        let key_str = key_str.unwrap();
                        let captured = match modifier {
                            Some(m) => format!("{}+{}", rdev_key_to_str(m).unwrap_or_default(), key_str),
                            None => key_str,
                        };

                        // Disarm before emitting so we don't fire twice.
                        armed2.store(false, Ordering::SeqCst);
                        let _ = app.emit(
                            "hotkey-captured",
                            serde_json::json!({"captured": captured, "which": which}),
                        );
                    }
                }
                rdev::EventType::KeyRelease(key) => {
                    let mut h = held2.lock().unwrap();
                    h.remove(&key);
                }
                _ => {}
            }
        });
    });

    Ok(())
}

fn is_modifier(k: rdev::Key) -> bool {
    matches!(
        k,
        rdev::Key::Alt
            | rdev::Key::AltGr
            | rdev::Key::ControlLeft
            | rdev::Key::ControlRight
            | rdev::Key::ShiftLeft
            | rdev::Key::ShiftRight
            | rdev::Key::MetaLeft
            | rdev::Key::MetaRight
    )
}

fn rdev_key_to_str(k: rdev::Key) -> Option<String> {
    Some(match k {
        rdev::Key::Alt => "Alt".to_string(),
        rdev::Key::AltGr => "AltGr".to_string(),
        rdev::Key::ControlLeft | rdev::Key::ControlRight => "Ctrl".to_string(),
        rdev::Key::ShiftLeft => "ShiftLeft".to_string(),
        rdev::Key::ShiftRight => "ShiftRight".to_string(),
        rdev::Key::MetaLeft => "MetaLeft".to_string(),
        rdev::Key::MetaRight => "MetaRight".to_string(),
        rdev::Key::KeyA => "KeyA".to_string(),
        rdev::Key::KeyB => "KeyB".to_string(),
        rdev::Key::KeyC => "KeyC".to_string(),
        rdev::Key::KeyD => "KeyD".to_string(),
        rdev::Key::KeyE => "KeyE".to_string(),
        rdev::Key::KeyF => "KeyF".to_string(),
        rdev::Key::KeyG => "KeyG".to_string(),
        rdev::Key::KeyH => "KeyH".to_string(),
        rdev::Key::KeyI => "KeyI".to_string(),
        rdev::Key::KeyJ => "KeyJ".to_string(),
        rdev::Key::KeyK => "KeyK".to_string(),
        rdev::Key::KeyL => "KeyL".to_string(),
        rdev::Key::KeyM => "KeyM".to_string(),
        rdev::Key::KeyN => "KeyN".to_string(),
        rdev::Key::KeyO => "KeyO".to_string(),
        rdev::Key::KeyP => "KeyP".to_string(),
        rdev::Key::KeyQ => "KeyQ".to_string(),
        rdev::Key::KeyR => "KeyR".to_string(),
        rdev::Key::KeyS => "KeyS".to_string(),
        rdev::Key::KeyT => "KeyT".to_string(),
        rdev::Key::KeyU => "KeyU".to_string(),
        rdev::Key::KeyV => "KeyV".to_string(),
        rdev::Key::KeyW => "KeyW".to_string(),
        rdev::Key::KeyX => "KeyX".to_string(),
        rdev::Key::KeyY => "KeyY".to_string(),
        rdev::Key::KeyZ => "KeyZ".to_string(),
        rdev::Key::F1 => "F1".to_string(),
        rdev::Key::F2 => "F2".to_string(),
        rdev::Key::F3 => "F3".to_string(),
        rdev::Key::F4 => "F4".to_string(),
        rdev::Key::F5 => "F5".to_string(),
        rdev::Key::F6 => "F6".to_string(),
        rdev::Key::F7 => "F7".to_string(),
        rdev::Key::F8 => "F8".to_string(),
        rdev::Key::F9 => "F9".to_string(),
        rdev::Key::F10 => "F10".to_string(),
        rdev::Key::F11 => "F11".to_string(),
        rdev::Key::F12 => "F12".to_string(),
        rdev::Key::Escape => "Esc".to_string(),
        rdev::Key::Return => "Return".to_string(),
        rdev::Key::Space => "Space".to_string(),
        rdev::Key::Tab => "Tab".to_string(),
        rdev::Key::Backspace => "Backspace".to_string(),
        rdev::Key::Delete => "Delete".to_string(),
        rdev::Key::Home => "Home".to_string(),
        rdev::Key::End => "End".to_string(),
        rdev::Key::UpArrow => "Up".to_string(),
        rdev::Key::DownArrow => "Down".to_string(),
        rdev::Key::LeftArrow => "Left".to_string(),
        rdev::Key::RightArrow => "Right".to_string(),
        rdev::Key::CapsLock => "CapsLock".to_string(),
        rdev::Key::Unknown(code) => format!("Unknown({code})"),
        _ => return None,
    })
}
