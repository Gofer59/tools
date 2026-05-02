use std::sync::mpsc;

pub enum HotkeyAction {
    Reselect,
    ToggleOnTop,
}

/// Spawn a background thread that listens for hotkeys and sends actions on `tx`.
///
/// `hk_reselect` / `hk_toggle_top`: (optional modifier key, action key).
pub fn spawn_hotkey_listener(
    tx: mpsc::Sender<HotkeyAction>,
    hk_reselect: (Option<rdev::Key>, rdev::Key),
    hk_toggle_top: (Option<rdev::Key>, rdev::Key),
) {
    #[cfg(target_os = "linux")]
    spawn_hotkey_listener_linux(tx, hk_reselect, hk_toggle_top);

    #[cfg(target_os = "windows")]
    spawn_hotkey_listener_windows(tx, hk_reselect, hk_toggle_top);
}

// ---------------------------------------------------------------------------
// Linux — rdev low-level hook
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn hotkey_matches(
    k: rdev::Key,
    hk: (Option<rdev::Key>, rdev::Key),
    held: &std::collections::HashSet<rdev::Key>,
) -> bool {
    k == hk.1 && hk.0.is_none_or(|m| held.contains(&m))
}

#[cfg(target_os = "linux")]
fn spawn_hotkey_listener_linux(
    action_tx: mpsc::Sender<HotkeyAction>,
    hk_reselect: (Option<rdev::Key>, rdev::Key),
    hk_toggle_top: (Option<rdev::Key>, rdev::Key),
) {
    use rdev::{listen, Event, EventType};
    use std::collections::HashSet;

    std::thread::spawn(move || {
        let mut held: HashSet<rdev::Key> = HashSet::new();

        if let Err(e) = listen(move |event: Event| {
            match event.event_type {
                EventType::KeyPress(k) => {
                    held.insert(k);
                    if hotkey_matches(k, hk_reselect, &held) {
                        let _ = action_tx.send(HotkeyAction::Reselect);
                    }
                    if hotkey_matches(k, hk_toggle_top, &held) {
                        let _ = action_tx.send(HotkeyAction::ToggleOnTop);
                    }
                }
                EventType::KeyRelease(k) => {
                    held.remove(&k);
                }
                _ => {}
            }
        }) {
            eprintln!("[threshold-filter] rdev listener error: {e:?}");
            eprintln!("[threshold-filter] Is the current user in the 'input' group?");
        }
    });
}

// ---------------------------------------------------------------------------
// Windows — Win32 RegisterHotKey
//
// rdev uses SetWindowsHookEx(WH_KEYBOARD_LL) which games with anti-cheat
// (e.g. Genshin Impact / HoYo Protect) actively block when in focus.
// RegisterHotKey posts WM_HOTKEY into the thread message queue at the
// Windows session level — games cannot intercept or block this path.
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn rdev_key_to_vk(k: rdev::Key) -> Option<u32> {
    use rdev::Key::*;
    Some(match k {
        KeyA => 0x41, KeyB => 0x42, KeyC => 0x43, KeyD => 0x44,
        KeyE => 0x45, KeyF => 0x46, KeyG => 0x47, KeyH => 0x48,
        KeyI => 0x49, KeyJ => 0x4A, KeyK => 0x4B, KeyL => 0x4C,
        KeyM => 0x4D, KeyN => 0x4E, KeyO => 0x4F, KeyP => 0x50,
        KeyQ => 0x51, KeyR => 0x52, KeyS => 0x53, KeyT => 0x54,
        KeyU => 0x55, KeyV => 0x56, KeyW => 0x57, KeyX => 0x58,
        KeyY => 0x59, KeyZ => 0x5A,
        F1 => 0x70,  F2 => 0x71,  F3 => 0x72,  F4 => 0x73,
        F5 => 0x74,  F6 => 0x75,  F7 => 0x76,  F8 => 0x77,
        F9 => 0x78,  F10 => 0x79, F11 => 0x7A, F12 => 0x7B,
        Escape => 0x1B, Space => 0x20, Return => 0x0D, Tab => 0x09,
        Backspace => 0x08, Delete => 0x2E, Home => 0x24, End => 0x23,
        UpArrow => 0x26, DownArrow => 0x28, LeftArrow => 0x25, RightArrow => 0x27,
        CapsLock => 0x14,
        _ => return None,
    })
}

#[cfg(target_os = "windows")]
fn rdev_key_to_mod(k: rdev::Key) -> u32 {
    use rdev::Key::*;
    match k {
        MetaLeft | MetaRight       => 0x0008, // MOD_WIN
        Alt | AltGr                => 0x0001, // MOD_ALT
        ControlLeft | ControlRight => 0x0002, // MOD_CONTROL
        ShiftLeft | ShiftRight     => 0x0004, // MOD_SHIFT
        _ => 0,
    }
}

#[cfg(target_os = "windows")]
fn spawn_hotkey_listener_windows(
    action_tx: mpsc::Sender<HotkeyAction>,
    hk_reselect: (Option<rdev::Key>, rdev::Key),
    hk_toggle_top: (Option<rdev::Key>, rdev::Key),
) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::RegisterHotKey;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

    std::thread::spawn(move || unsafe {
        const MOD_NOREPEAT: u32 = 0x4000;

        let try_register = |id: i32, hk: (Option<rdev::Key>, rdev::Key)| {
            if let Some(vk) = rdev_key_to_vk(hk.1) {
                let mods = hk.0.map(rdev_key_to_mod).unwrap_or(0) | MOD_NOREPEAT;
                if RegisterHotKey(0, id, mods, vk) == 0 {
                    eprintln!(
                        "[threshold-filter] RegisterHotKey id={id} failed \
                         (hotkey may already be in use by another app)"
                    );
                }
            } else {
                eprintln!("[threshold-filter] Key {:?} has no Win32 VK mapping", hk.1);
            }
        };

        try_register(1, hk_reselect);
        try_register(2, hk_toggle_top);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            if msg.message == WM_HOTKEY {
                match msg.wParam as i32 {
                    1 => { let _ = action_tx.send(HotkeyAction::Reselect); }
                    2 => { let _ = action_tx.send(HotkeyAction::ToggleOnTop); }
                    _ => {}
                }
            }
        }
    });
}
