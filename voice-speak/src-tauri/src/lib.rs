pub mod audio;
pub mod clipboard;
pub mod commands;
pub mod config;
pub mod events;
pub mod hotkey;
pub mod paths;
pub mod piper;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter, Listener, Manager};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

/// Commands sent over the audio channel to the dedicated audio thread.
pub enum AudioCmd {
    Play(Vec<piper::PcmChunk>),
    Stop,
}

pub struct AppState {
    pub config: Arc<RwLock<config::Config>>,
    pub hotkey_ctrl: Arc<Mutex<hotkey::HotkeyHandle>>,
    pub daemon_ctrl: Arc<Mutex<piper::DaemonHandle>>,
    pub download_map: Arc<Mutex<HashMap<String, CancellationToken>>>,
    pub test_hotkey_armed: Arc<AtomicBool>,
    pub audio_tx: std::sync::mpsc::SyncSender<AudioCmd>,
}

pub fn run() {
    // Spawn the audio thread before building the Tauri app so that audio_tx
    // can be moved into the setup closure and stored in AppState.
    // AudioPlayer holds rodio's OutputStream which is !Send, so it must live
    // entirely within this thread. We give the thread its own single-threaded
    // tokio runtime to drive the async play/stop methods.
    let (audio_tx, audio_rx) = std::sync::mpsc::sync_channel::<AudioCmd>(4);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("audio thread tokio runtime");

        rt.block_on(async move {
            let player = audio::AudioPlayer::new().expect("AudioPlayer init");
            loop {
                // recv() blocks the thread until a message arrives; safe in
                // block_on because we yield control back to tokio between awaits.
                match audio_rx.recv() {
                    Ok(AudioCmd::Play(chunks)) => {
                        let _ = player.play(chunks).await;
                    }
                    Ok(AudioCmd::Stop) => {
                        player.stop().await;
                    }
                    Err(_) => break, // channel closed — app is shutting down
                }
            }
        });
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(move |app| {
            let handle = app.handle().clone();
            let audio_tx_for_state = audio_tx.clone();

            tauri::async_runtime::block_on(async move {
                let dir = handle
                    .path()
                    .app_local_data_dir()
                    .expect("app_local_data_dir");
                let cfg = config::load_or_default(&dir);
                let model_dir = dir.join("models");
                let script = paths::daemon_script(&handle);

                let daemon = match piper::spawn(&cfg.python_bin, &script, &model_dir).await {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("[voice-speak] piper daemon failed to start: {e}");
                        panic!("piper daemon required: {e}");
                    }
                };

                let hk = hotkey::register(&handle, &cfg.hotkey).expect("hotkey register");

                let state = AppState {
                    config: Arc::new(RwLock::new(cfg)),
                    hotkey_ctrl: Arc::new(Mutex::new(hk)),
                    daemon_ctrl: Arc::new(Mutex::new(daemon)),
                    download_map: Arc::new(Mutex::new(HashMap::new())),
                    test_hotkey_armed: Arc::new(AtomicBool::new(false)),
                    audio_tx: audio_tx_for_state,
                };
                handle.manage(state);

                let _ = handle.emit("daemon-ready", serde_json::json!({"ready": true}));
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

            // Wire the hotkey toggle pipeline
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
        ])
        .run(tauri::generate_context!())
        .expect("tauri run failed");
}

/// Listen for `hotkey-triggered` events and toggle TTS speak/stop.
fn wire_hotkey_pipeline(app: AppHandle) {
    // Track whether we are currently speaking so each press toggles.
    let speaking = Arc::new(AtomicBool::new(false));

    app.clone().listen("hotkey-triggered", move |ev| {
        let v: serde_json::Value = match serde_json::from_str(ev.payload()) {
            Ok(v) => v,
            Err(_) => return,
        };
        // Only react to the key-press edge; ignore release.
        if v["state"].as_str() != Some("pressed") {
            return;
        }

        let app2 = app.clone();
        let speaking2 = speaking.clone();

        tauri::async_runtime::spawn(async move {
            let state = match app2.try_state::<AppState>() {
                Some(s) => s,
                None => return,
            };

            let currently_speaking = speaking2.load(Ordering::SeqCst);

            if currently_speaking {
                // Stop playback
                let _ = state.audio_tx.send(AudioCmd::Stop);
                let daemon = state.daemon_ctrl.lock().await;
                let _ = piper::stop(&daemon, "tts").await;
                speaking2.store(false, Ordering::SeqCst);
                let _ = app2.emit("tts-state", serde_json::json!({"state": "stopped"}));
            } else {
                // Read selected text, synthesize, play
                let text = match clipboard::read_selection() {
                    Ok(t) if !t.is_empty() => t,
                    _ => return,
                };

                speaking2.store(true, Ordering::SeqCst);
                let _ = app2.emit("tts-state", serde_json::json!({"state": "speaking"}));

                let cfg = state.config.read().await.clone();
                let chunks = {
                    let daemon = state.daemon_ctrl.lock().await;
                    piper::speak(
                        &daemon,
                        &text,
                        &cfg.voice,
                        cfg.speed,
                        cfg.noise_scale,
                        cfg.noise_w_scale,
                        "tts",
                    )
                    .await
                };

                match chunks {
                    Ok(pcm) => {
                        let _ = state.audio_tx.send(AudioCmd::Play(pcm));
                    }
                    Err(e) => {
                        eprintln!("[voice-speak] speak error: {e}");
                        speaking2.store(false, Ordering::SeqCst);
                        let _ = app2.emit(
                            "tts-state",
                            serde_json::json!({"state": "error", "message": e.to_string()}),
                        );
                        return;
                    }
                }

                // After audio is queued, mark done
                speaking2.store(false, Ordering::SeqCst);
                let _ = app2.emit("tts-state", serde_json::json!({"state": "done"}));
            }
        });
    });
}
