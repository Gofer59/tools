mod capture;
mod config;
mod processing;
mod ui;

use std::collections::HashSet;
use std::sync::mpsc;

use rdev::{listen, Event, EventType, Key};

use crate::ui::HotkeyAction;

// ---------------------------------------------------------------------------
// Hotkey matching
// ---------------------------------------------------------------------------

fn hotkey_matches(k: Key, hk: (Option<Key>, Key), held: &HashSet<Key>) -> bool {
    k == hk.1 && hk.0.is_none_or(|m| held.contains(&m))
}

fn spawn_hotkey_listener(
    action_tx: mpsc::Sender<HotkeyAction>,
    hk_reselect: (Option<Key>, Key),
    hk_toggle_top: (Option<Key>, Key),
) {
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
            #[cfg(target_os = "linux")]
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
