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
use std::sync::atomic::AtomicBool;

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
                let dir = handle
                    .path()
                    .app_local_data_dir()
                    .expect("app_local_data_dir");
                let cfg = config::load_or_default(&dir);
                let model_dir = dir.join("models");
                let script = paths::daemon_script(&handle);

                let daemon = match whisper::spawn(
                    &cfg.python_bin,
                    &script,
                    &cfg.whisper_model,
                    &cfg.compute_type,
                    &model_dir,
                )
                .await
                {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("[voice-prompt] whisper daemon failed to start: {e}");
                        // App still starts; transcription will fail gracefully
                        panic!("whisper daemon required: {e}");
                    }
                };

                let hk = hotkey::register(&handle, &cfg.push_to_talk_key)
                    .expect("hotkey register");

                let state = AppState {
                    config: Arc::new(RwLock::new(cfg)),
                    hotkey_ctrl: Arc::new(Mutex::new(hk)),
                    daemon_ctrl: Arc::new(Mutex::new(daemon)),
                    download_map: Arc::new(Mutex::new(HashMap::new())),
                    test_hotkey_armed: Arc::new(AtomicBool::new(false)),
                    recording_stop: Arc::new(AtomicBool::new(false)),
                    record_handle: Arc::new(Mutex::new(None)),
                };
                handle.manage(state);

                let _ = handle.emit("daemon-ready", serde_json::json!({"model": "loaded"}));
            });

            // System tray
            let menu = Menu::with_items(
                app,
                &[
                    &MenuItem::with_id(app, "show", "Show", true, None::<&str>)?,
                    &MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?,
                ],
            )?;
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

            // Wire hotkey events to the recording pipeline
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
        .run(tauri::generate_context!())
        .expect("tauri run failed");
}

fn wire_hotkey_pipeline(app: AppHandle) {
    use tauri::Listener;
    let app_for_closure = app.clone();
    app.listen("hotkey-triggered", move |ev| {
        let app2 = app_for_closure.clone();
        tauri::async_runtime::spawn(async move {
            handle_hotkey_event(app2, ev.payload()).await;
        });
    });
}

async fn handle_hotkey_event(app: AppHandle, payload: &str) {
    let v: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return,
    };
    let state = match app.try_state::<AppState>() {
        Some(s) => s,
        None => return,
    };

    let state_str = v["state"].as_str().unwrap_or("");
    match state_str {
        "pressed" => {
            state.recording_stop.store(false, std::sync::atomic::Ordering::SeqCst);
            let stop = state.recording_stop.clone();
            match audio_in::start_recording(stop) {
                Ok(handle) => {
                    *state.record_handle.lock().await = Some(handle);
                }
                Err(e) => {
                    eprintln!("[voice-prompt] recording start failed: {e}");
                }
            }
        }
        "released" => {
            state.recording_stop.store(true, std::sync::atomic::Ordering::SeqCst);

            let handle_opt = state.record_handle.lock().await.take();
            let Some(handle) = handle_opt else { return };

            let wav = match tauri::async_runtime::spawn_blocking(move || {
                audio_in::finish_recording(handle)
            })
            .await
            {
                Ok(Ok(f)) => f,
                Ok(Err(e)) => { eprintln!("[voice-prompt] finish_recording: {e}"); return; }
                Err(e) => { eprintln!("[voice-prompt] recording thread join: {e}"); return; }
            };

            let (language, vad) = {
                let cfg = state.config.read().await;
                (cfg.language.clone(), cfg.vad_filter)
            };

            let wav_path = wav.path().to_path_buf();
            let mut daemon = state.daemon_ctrl.lock().await;
            match whisper::transcribe(&mut *daemon, &wav_path, &language, vad).await {
                Ok((text, _ms)) => {
                    drop(daemon);
                    drop(wav);
                    if let Err(e) = injection::inject(&text) {
                        eprintln!("[voice-prompt] inject: {e}");
                    }
                }
                Err(e) => {
                    eprintln!("[voice-prompt] transcribe: {e}");
                }
            }
        }
        _ => {}
    }
}
