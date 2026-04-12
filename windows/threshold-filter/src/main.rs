mod capture;
mod config;
mod processing;
mod ui;

use std::sync::mpsc;

use rdev::Key;

use crate::ui::HotkeyAction;

// ---------------------------------------------------------------------------
// Windows hotkey listener — Win32 RegisterHotKey
//
// rdev uses SetWindowsHookEx(WH_KEYBOARD_LL) which games with anti-cheat
// (e.g. Genshin Impact / HoYo Protect) actively block when in focus.
// RegisterHotKey posts WM_HOTKEY into the thread message queue at the
// Windows session level — games cannot intercept or block this path.
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn rdev_key_to_vk(k: Key) -> Option<u32> {
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
        Escape => 0x1B, Space => 0x20,  Return => 0x0D, Tab => 0x09,
        Backspace => 0x08, Delete => 0x2E, Home => 0x24, End => 0x23,
        UpArrow => 0x26, DownArrow => 0x28, LeftArrow => 0x25, RightArrow => 0x27,
        CapsLock => 0x14,
        _ => return None,
    })
}

#[cfg(target_os = "windows")]
fn rdev_key_to_mod(k: Key) -> u32 {
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
fn spawn_hotkey_listener(
    action_tx: mpsc::Sender<HotkeyAction>,
    hk_reselect: (Option<Key>, Key),
    hk_toggle_top: (Option<Key>, Key),
) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::RegisterHotKey;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

    std::thread::spawn(move || unsafe {
        const MOD_NOREPEAT: u32 = 0x4000;

        let try_register = |id: i32, hk: (Option<Key>, Key)| {
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

// ---------------------------------------------------------------------------
// Linux hotkey listener — rdev low-level hook
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn hotkey_matches(k: Key, hk: (Option<Key>, Key), held: &std::collections::HashSet<Key>) -> bool {
    k == hk.1 && hk.0.is_none_or(|m| held.contains(&m))
}

#[cfg(target_os = "linux")]
fn spawn_hotkey_listener(
    action_tx: mpsc::Sender<HotkeyAction>,
    hk_reselect: (Option<Key>, Key),
    hk_toggle_top: (Option<Key>, Key),
) {
    use rdev::{listen, Event, EventType};
    use std::collections::HashSet;

    std::thread::spawn(move || {
        let mut held: HashSet<Key> = HashSet::new();

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
// Main
// ---------------------------------------------------------------------------

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cfg = config::load_config()?;
    let hk_reselect = config::parse_hotkey(&cfg.hotkeys.region_select)?;
    let hk_toggle_top = config::parse_hotkey(&cfg.hotkeys.toggle_on_top)?;

    let always_on_top = cfg.display.always_on_top;
    let default_threshold = cfg.display.default_threshold;
    let invert = cfg.display.invert;

    // Global hotkey listener
    let (action_tx, action_rx) = mpsc::channel::<HotkeyAction>();
    spawn_hotkey_listener(action_tx, hk_reselect, hk_toggle_top);

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

    let key_reselect_name = cfg.hotkeys.region_select.clone();
    let key_toggle_top_name = cfg.hotkeys.toggle_on_top.clone();

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
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))
}
