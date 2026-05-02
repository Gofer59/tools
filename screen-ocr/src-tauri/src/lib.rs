pub mod commands;
mod capture;
mod clipboard;
mod config;
mod display;
mod hotkey;
mod ocr;
mod paths;
mod pipeline;
mod region;
mod tts;
mod typing;

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use tauri::{AppHandle, Manager};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;

use crate::display::DisplayServer;

pub struct AppState {
    pub config: Arc<RwLock<config::Config>>,
    pub hotkey_quick: Arc<Mutex<hotkey::HotkeyHandle>>,
    pub hotkey_select: Arc<Mutex<hotkey::HotkeyHandle>>,
    pub hotkey_stop: Arc<Mutex<hotkey::HotkeyHandle>>,
    pub tts_child: Arc<Mutex<Option<std::process::Child>>>,
    pub last_region: Arc<Mutex<Option<region::Region>>>,
    pub display: DisplayServer,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                setup(handle).await;
            });

            let menu = Menu::with_items(app, &[
                &MenuItem::with_id(app, "show", "Show", true, None::<&str>)?,
                &MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?,
            ])?;
            let _ = TrayIconBuilder::new()
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, ev| match ev.id().as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            wire_hotkey_pipeline(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::config::get_config,
            commands::config::update_config,
            commands::hotkey::test_hotkey,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("tauri run failed");
}

async fn setup(app: AppHandle) {
    let dir = app
        .path()
        .app_local_data_dir()
        .expect("app_local_data_dir");

    let cfg = config::load_or_default(&dir);
    let display = display::detect();

    let hk_quick = hotkey::register(&app, &cfg.hotkey_quick_capture, "quick")
        .expect("register hotkey_quick_capture");
    let hk_select = hotkey::register(&app, &cfg.hotkey_select_region, "select")
        .expect("register hotkey_select_region");
    let hk_stop = hotkey::register(&app, &cfg.hotkey_stop_tts, "stop")
        .expect("register hotkey_stop_tts");

    let state = AppState {
        config: Arc::new(RwLock::new(cfg)),
        hotkey_quick: Arc::new(Mutex::new(hk_quick)),
        hotkey_select: Arc::new(Mutex::new(hk_select)),
        hotkey_stop: Arc::new(Mutex::new(hk_stop)),
        tts_child: Arc::new(Mutex::new(None)),
        last_region: Arc::new(Mutex::new(None)),
        display,
    };

    app.manage(state);
    eprintln!("[screen-ocr] ready (display={:?})", display);
}

fn wire_hotkey_pipeline(app: AppHandle) {
    use tauri::Listener;
    let app2 = app.clone();
    app.listen("hotkey-triggered", move |ev| {
        let app3 = app2.clone();
        let payload = ev.payload().to_string();
        tauri::async_runtime::spawn(async move {
            handle_hotkey_event(app3, &payload).await;
        });
    });
}

async fn handle_hotkey_event(app: AppHandle, payload: &str) {
    let v: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return,
    };

    // Only handle "pressed" events; these hotkeys have no "released" action.
    if v["state"].as_str().unwrap_or("") != "pressed" {
        return;
    }

    let action = match v["action"].as_str() {
        Some(a) => a.to_string(),
        None => return,
    };

    match action.as_str() {
        "quick" => {
            eprintln!("[screen-ocr] hotkey: quick capture");
            tauri::async_runtime::spawn(async move {
                pipeline::run(app, pipeline::Mode::Quick).await;
            });
        }
        "select" => {
            eprintln!("[screen-ocr] hotkey: select region");
            tauri::async_runtime::spawn(async move {
                pipeline::run(app, pipeline::Mode::Select).await;
            });
        }
        "stop" => {
            eprintln!("[screen-ocr] hotkey: stop TTS");
            if let Some(state) = app.try_state::<AppState>() {
                let mut guard = state.tts_child.lock().await;
                if let Some(ref mut child) = *guard {
                    match child.try_wait() {
                        Ok(Some(_)) => eprintln!("[screen-ocr] TTS already finished."),
                        _ => tts::kill(child),
                    }
                    *guard = None;
                }
            }
        }
        other => {
            eprintln!("[screen-ocr] unknown hotkey action: {other}");
        }
    }
}
