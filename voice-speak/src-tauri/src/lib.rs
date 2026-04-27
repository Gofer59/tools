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
    pub audio_tx: tokio::sync::mpsc::Sender<AudioCmd>,
    pub is_speaking: Arc<AtomicBool>,
    /// Last PRIMARY content we acted on — used to detect fresh selections.
    pub last_primary: Arc<Mutex<String>>,
}

pub fn run() {
    // Spawn the audio thread before building the Tauri app so that audio_tx
    // can be moved into the setup closure and stored in AppState.
    // AudioPlayer holds rodio's OutputStream which is !Send, so it must live
    // entirely within this thread. We give the thread its own single-threaded
    // tokio runtime to drive the async play/stop methods.
    let (audio_tx, mut audio_rx) = tokio::sync::mpsc::channel::<AudioCmd>(4);
    let is_speaking_audio = Arc::new(AtomicBool::new(false));
    let is_speaking_state = is_speaking_audio.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("audio thread tokio runtime");

        rt.block_on(async move {
            let player = audio::AudioPlayer::new().expect("AudioPlayer init");
            loop {
                match audio_rx.recv().await {
                    Some(AudioCmd::Play(chunks)) => {
                        is_speaking_audio.store(true, Ordering::SeqCst);
                        let _ = player.play(chunks).await;
                        // Wait until natural end OR a Stop/Play arrives.
                        loop {
                            tokio::select! {
                                cmd = audio_rx.recv() => {
                                    match cmd {
                                        Some(AudioCmd::Stop) | None => {
                                            player.stop().await;
                                            break;
                                        }
                                        Some(AudioCmd::Play(new_chunks)) => {
                                            let _ = player.play(new_chunks).await;
                                        }
                                    }
                                }
                                _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
                                    if !player.is_playing().await {
                                        break;
                                    }
                                }
                            }
                        }
                        is_speaking_audio.store(false, Ordering::SeqCst);
                    }
                    Some(AudioCmd::Stop) => {
                        player.stop().await;
                    }
                    None => break,
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

                // Seed last_primary with whatever is currently in PRIMARY so the
                // first hotkey press doesn't treat pre-existing stale text as fresh.
                let initial_primary = clipboard::read_primary().unwrap_or_default();

                let state = AppState {
                    config: Arc::new(RwLock::new(cfg)),
                    hotkey_ctrl: Arc::new(Mutex::new(hk)),
                    daemon_ctrl: Arc::new(Mutex::new(daemon)),
                    download_map: Arc::new(Mutex::new(HashMap::new())),
                    test_hotkey_armed: Arc::new(AtomicBool::new(false)),
                    audio_tx: audio_tx_for_state,
                    is_speaking: is_speaking_state,
                    last_primary: Arc::new(Mutex::new(initial_primary)),
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
    app.clone().listen("hotkey-triggered", move |ev| {
        let v: serde_json::Value = match serde_json::from_str(ev.payload()) {
            Ok(v) => v,
            Err(_) => return,
        };
        if v["state"].as_str() != Some("pressed") {
            return;
        }

        let app2 = app.clone();
        tauri::async_runtime::spawn(async move {
            let state = match app2.try_state::<AppState>() {
                Some(s) => s,
                None => return,
            };

            if state.is_speaking.load(Ordering::SeqCst) {
                // Second press — stop playback. The audio thread clears is_speaking when done.
                let _ = state.audio_tx.send(AudioCmd::Stop).await;
                state.is_speaking.store(false, Ordering::SeqCst);
                let _ = app2.emit("tts-state", serde_json::json!({"state": "stopped"}));
                return;
            }

            // First press — pick whichever source was updated most recently.
            // PRIMARY changes when the user makes a new selection; CLIPBOARD
            // changes on Ctrl+C. We track the last PRIMARY we saw: if it
            // changed, the selection is newer; if it's the same, the clipboard
            // is newer (or the selection is stale).
            let primary = clipboard::read_primary().unwrap_or_default();
            let clipboard_text = clipboard::read_clipboard().unwrap_or_default();

            let text = {
                let mut last = state.last_primary.lock().await;
                let primary_is_fresh = !primary.is_empty() && primary != *last;
                if primary_is_fresh {
                    *last = primary.clone();
                    primary
                } else if !clipboard_text.is_empty() {
                    clipboard_text
                } else if !primary.is_empty() {
                    // Nothing in clipboard — read whatever primary has.
                    primary
                } else {
                    eprintln!("[voice-speak] nothing to read");
                    return;
                }
            };

            state.is_speaking.store(true, Ordering::SeqCst);
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
                    // Queue audio; audio thread owns is_speaking and clears it when done.
                    let _ = state.audio_tx.send(AudioCmd::Play(pcm)).await;
                }
                Err(e) => {
                    eprintln!("[voice-speak] speak error: {e}");
                    state.is_speaking.store(false, Ordering::SeqCst);
                    let _ = app2.emit(
                        "tts-state",
                        serde_json::json!({"state": "error", "message": e.to_string()}),
                    );
                }
            }
        });
    });
}
