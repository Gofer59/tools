mod audio_in;
pub mod commands;
mod config;
mod events;
mod hotkey;
mod injection;
mod paths;
mod whisper;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter, Manager};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

pub struct AppState {
    pub config: Arc<RwLock<config::Config>>,
    pub hotkey_ctrl: Arc<Mutex<hotkey::HotkeyHandle>>,
    pub daemon_ctrl: Arc<Mutex<whisper::DaemonHandle>>,
    pub download_map: Arc<Mutex<HashMap<String, CancellationToken>>>,
    pub test_hotkey_armed: Arc<AtomicBool>,
    pub recording_stop: Arc<AtomicBool>,
    pub record_handle: Arc<Mutex<Option<audio_in::RecordHandle>>>,
    pub target_window: Arc<Mutex<Option<String>>>,
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
                let dir = handle.path().app_local_data_dir().expect("app_local_data_dir");
                let cfg = config::load_or_default(&dir);
                let model_dir = dir.join("models");
                let script = paths::daemon_script(&handle);

                let daemon = match whisper::spawn(
                    &cfg.python_bin, &script, &cfg.whisper_model,
                    &cfg.compute_type, &model_dir, "cpu",
                ).await {
                    Ok(d) => d,
                    Err(e) => panic!("whisper daemon required: {e}"),
                };

                let hk = hotkey::register(&handle, &cfg.push_to_talk_key).expect("hotkey register");

                let state = AppState {
                    config: Arc::new(RwLock::new(cfg)),
                    hotkey_ctrl: Arc::new(Mutex::new(hk)),
                    daemon_ctrl: Arc::new(Mutex::new(daemon)),
                    download_map: Arc::new(Mutex::new(HashMap::new())),
                    test_hotkey_armed: Arc::new(AtomicBool::new(false)),
                    recording_stop: Arc::new(AtomicBool::new(false)),
                    record_handle: Arc::new(Mutex::new(None)),
                    target_window: Arc::new(Mutex::new(None)),
                };
                handle.manage(state);
                let _ = handle.emit("daemon-ready", serde_json::json!({"model": "loaded"}));
            });

            let menu = Menu::with_items(app, &[
                &MenuItem::with_id(app, "show", "Show", true, None::<&str>)?,
                &MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?,
            ])?;
            let _ = TrayIconBuilder::new()
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, ev| match ev.id().as_ref() {
                    "show" => { if let Some(w) = app.get_webview_window("main") { let _ = w.show(); } }
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
            commands::models::list_catalog_models,
            commands::models::list_local_models,
            commands::models::download_model,
            commands::models::cancel_download,
            commands::models::add_custom_model,
            commands::models::delete_local_model,
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
        Ok(v) => v, Err(_) => return,
    };
    let state = match app.try_state::<AppState>() {
        Some(s) => s, None => return,
    };

    match v["state"].as_str().unwrap_or("") {
        "pressed" => {
            eprintln!("[voice-prompt] hotkey pressed — starting recording");
            state.recording_stop.store(false, Ordering::SeqCst);

            let win_id = capture_active_window();
            eprintln!("[voice-prompt] target window: {:?}", win_id);
            *state.target_window.lock().await = win_id;

            match audio_in::start_recording(state.recording_stop.clone()) {
                Ok(handle) => {
                    *state.record_handle.lock().await = Some(handle);
                    eprintln!("[voice-prompt] recording started");
                }
                Err(e) => eprintln!("[voice-prompt] recording start failed: {e}"),
            }
        }

        "released" => {
            eprintln!("[voice-prompt] hotkey released — finishing recording");

            state.recording_stop.store(true, Ordering::SeqCst);

            let handle_opt = state.record_handle.lock().await.take();
            let Some(handle) = handle_opt else {
                eprintln!("[voice-prompt] no active recording handle — skipping");
                return;
            };

            let wav = match tauri::async_runtime::spawn_blocking(move || {
                audio_in::finish_recording(handle)
            }).await {
                Ok(Ok(f)) => f,
                Ok(Err(e)) => { eprintln!("[voice-prompt] finish_recording: {e}"); return; }
                Err(e) => { eprintln!("[voice-prompt] recording thread join: {e}"); return; }
            };
            eprintln!("[voice-prompt] wav written: {}", wav.path().display());

            let (language, vad) = {
                let cfg = state.config.read().await;
                (cfg.language.clone(), cfg.vad_filter)
            };
            let win_id = state.target_window.lock().await.clone();

            let daemon = state.daemon_ctrl.lock().await;
            match whisper::transcribe(&daemon, wav.path(), &language, vad).await {
                Ok((text, ms)) => {
                    eprintln!("[voice-prompt] transcribed ({ms} ms): {text:?}");
                    drop(daemon); drop(wav);
                    if let Err(e) = injection::inject(&text, win_id.as_deref()) {
                        eprintln!("[voice-prompt] inject: {e}");
                    }
                }
                Err(e) => eprintln!("[voice-prompt] transcribe: {e}"),
            }
        }
        _ => {}
    }
}

fn capture_active_window() -> Option<String> {
    #[cfg(not(target_os = "windows"))]
    {
        if std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland")
            || std::env::var("WAYLAND_DISPLAY").is_ok()
        {
            return None;
        }
        std::process::Command::new("xdotool")
            .arg("getactivewindow")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }
    #[cfg(target_os = "windows")]
    None
}
