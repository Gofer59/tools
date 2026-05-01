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
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use tauri::{AppHandle, Emitter, Manager};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::whisper::StreamEvent;

pub struct AppState {
    pub config: Arc<RwLock<config::Config>>,
    pub hotkey_ctrl: Arc<Mutex<hotkey::HotkeyHandle>>,
    pub daemon_ctrl: Arc<Mutex<whisper::DaemonHandle>>,
    pub tiny_daemon_ctrl: Arc<Mutex<Option<whisper::DaemonHandle>>>,
    pub download_map: Arc<Mutex<HashMap<String, CancellationToken>>>,
    pub test_hotkey_armed: Arc<AtomicBool>,
    pub recording_stop: Arc<AtomicBool>,
    pub record_handle: Arc<Mutex<Option<audio_in::RecordHandle>>>,
    pub target_window: Arc<Mutex<Option<String>>>,
    pub live_preview_len: Arc<AtomicUsize>,
    pub last_partial: Arc<Mutex<String>>,
    pub streaming_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub streaming_release: Arc<AtomicBool>,
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
                    &cfg.compute_type, &model_dir, &cfg.large_device,
                ).await {
                    Ok(d) => d,
                    Err(e) => panic!("whisper daemon required: {e}"),
                };

                let tiny_daemon = if cfg.preview_mode != "none" {
                    match whisper::spawn(
                        &cfg.python_bin, &script, &cfg.tiny_model,
                        &cfg.compute_type, &model_dir, &cfg.tiny_device,
                    ).await {
                        Ok(d) => { eprintln!("[voice-prompt] tiny daemon ready ({})", cfg.tiny_model); Some(d) }
                        Err(e) => { eprintln!("[voice-prompt] tiny daemon failed: {e}"); None }
                    }
                } else { None };

                let hk = hotkey::register(&handle, &cfg.push_to_talk_key).expect("hotkey register");

                let state = AppState {
                    config: Arc::new(RwLock::new(cfg)),
                    hotkey_ctrl: Arc::new(Mutex::new(hk)),
                    daemon_ctrl: Arc::new(Mutex::new(daemon)),
                    tiny_daemon_ctrl: Arc::new(Mutex::new(tiny_daemon)),
                    download_map: Arc::new(Mutex::new(HashMap::new())),
                    test_hotkey_armed: Arc::new(AtomicBool::new(false)),
                    recording_stop: Arc::new(AtomicBool::new(false)),
                    record_handle: Arc::new(Mutex::new(None)),
                    target_window: Arc::new(Mutex::new(None)),
                    live_preview_len: Arc::new(AtomicUsize::new(0)),
                    last_partial: Arc::new(Mutex::new(String::new())),
                    streaming_task: Arc::new(Mutex::new(None)),
                    streaming_release: Arc::new(AtomicBool::new(false)),
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
            state.live_preview_len.store(0, Ordering::SeqCst);
            state.streaming_release.store(false, Ordering::SeqCst);
            *state.last_partial.lock().await = String::new();

            let win_id = capture_active_window();
            eprintln!("[voice-prompt] target window: {:?}", win_id);
            *state.target_window.lock().await = win_id.clone();

            match audio_in::start_recording(state.recording_stop.clone()) {
                Ok(handle) => {
                    let (language, vad, preview_mode, hop_ms) = {
                        let cfg = state.config.read().await;
                        (cfg.language.clone(), cfg.vad_filter, cfg.preview_mode.clone(),
                         cfg.live_hop_ms)
                    };

                    if preview_mode != "none" {
                        let overlay_mode = preview_mode == "overlay";
                        let task = spawn_streaming_pipeline(
                            app.clone(),
                            state.tiny_daemon_ctrl.clone(),
                            handle.full.clone(),
                            handle.sample_rate,
                            handle.channels,
                            language,
                            vad,
                            hop_ms,
                            win_id,
                            state.recording_stop.clone(),
                            state.streaming_release.clone(),
                            state.live_preview_len.clone(),
                            state.last_partial.clone(),
                            overlay_mode,
                        );
                        *state.streaming_task.lock().await = Some(task);
                    }

                    *state.record_handle.lock().await = Some(handle);
                    eprintln!("[voice-prompt] recording started");
                }
                Err(e) => eprintln!("[voice-prompt] recording start failed: {e}"),
            }
        }

        "released" => {
            eprintln!("[voice-prompt] hotkey released — finishing recording");

            // 1. Gate the streaming consumer so any in-flight partials are dropped.
            state.streaming_release.store(true, Ordering::SeqCst);
            // 2. Stop producer + audio thread.
            state.recording_stop.store(true, Ordering::SeqCst);

            // 3. If a streaming task is running, send stream_stop to unblock the
            //    consumer's stdout read, then await the task (its consumer drains
            //    daemon to Idle, awaits its own producer).
            let task_opt = state.streaming_task.lock().await.take();
            if let Some(task) = task_opt {
                // Clone the stdin Arc out of the outer Option<DaemonHandle> mutex
                // and drop the outer guard BEFORE awaiting the streaming task —
                // the streaming task's EOF cleanup also locks tiny_daemon_ctrl,
                // so holding the outer guard across .await would deadlock.
                let stdin_opt = {
                    let g = state.tiny_daemon_ctrl.lock().await;
                    g.as_ref().map(|d| d.stdin.clone())
                };
                if let Some(stdin) = stdin_opt {
                    use tokio::io::AsyncWriteExt;
                    let mut s = stdin.lock().await;
                    let _ = s.write_all(b"{\"cmd\":\"stream_stop\"}\n").await;
                    let _ = s.flush().await;
                }
                let _ = task.await;
            }

            // 4. Now safe to read live_preview_len: no concurrent writers remain.
            let preview_len = state.live_preview_len.swap(0, Ordering::SeqCst);

            // 5. Finish recording (drain audio thread + write final WAV).
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

            let (language, vad, preview_mode) = {
                let cfg = state.config.read().await;
                (cfg.language.clone(), cfg.vad_filter, cfg.preview_mode.clone())
            };
            let win_id = state.target_window.lock().await.clone();

            if preview_mode == "none" {
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
            } else {
                let overlay_mode = preview_mode == "overlay";
                let wav_path = wav.path().to_path_buf();

                let daemon = state.daemon_ctrl.lock().await;
                match whisper::transcribe(&daemon, &wav_path, &language, vad).await {
                    Ok((final_text, large_ms)) => {
                        eprintln!("[voice-prompt] large final ({large_ms} ms): {final_text:?}");
                        drop(daemon); drop(wav);
                        let _ = app.emit("final-transcript", serde_json::json!({"text": final_text}));

                        if overlay_mode {
                            hide_overlay(&app);
                            if let Err(e) = injection::inject(&final_text, win_id.as_deref()) {
                                eprintln!("[voice-prompt] final inject: {e}");
                            }
                        } else {
                            if preview_len > 0 {
                                eprintln!("[voice-prompt] deleting live preview ({preview_len} chars)");
                                if let Err(e) = injection::delete_chars(preview_len, win_id.as_deref()) {
                                    eprintln!("[voice-prompt] delete_chars: {e}");
                                }
                            }
                            if let Err(e) = injection::inject(&final_text, win_id.as_deref()) {
                                eprintln!("[voice-prompt] final inject: {e}");
                            }
                        }
                        injection::release_modifiers();
                    }
                    Err(e) => {
                        eprintln!("[voice-prompt] large transcribe failed: {e}");
                        drop(daemon); drop(wav);
                        if !overlay_mode && preview_len > 0 {
                            let _ = injection::delete_chars(preview_len, win_id.as_deref());
                        }
                        if overlay_mode { hide_overlay(&app); }
                        injection::release_modifiers();
                    }
                }
            }
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_streaming_pipeline(
    app: AppHandle,
    tiny_daemon: Arc<Mutex<Option<whisper::DaemonHandle>>>,
    full_audio: Arc<std::sync::Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
    language: String,
    vad: bool,
    hop_ms: u32,
    win_id: Option<String>,
    stop_flag: Arc<AtomicBool>,
    release_gate: Arc<AtomicBool>,
    preview_len: Arc<AtomicUsize>,
    last_partial: Arc<Mutex<String>>,
    overlay_mode: bool,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Take split locks from the daemon handle so producer and consumer
        // never contend on the outer Option<DaemonHandle>.
        let (stdin_lock, stdout_lock) = {
            let mut g = tiny_daemon.lock().await;
            let Some(d) = g.as_mut() else {
                eprintln!("[voice-prompt] streaming: tiny daemon absent");
                return;
            };
            // window_seconds is informational on the daemon side; pass 0.0.
            if let Err(e) = whisper::stream_start(
                d, &language, vad, sample_rate, 0.0, hop_ms,
            ).await {
                eprintln!("[voice-prompt] stream_start failed: {e}");
                return;
            }
            (d.stdin.clone(), d.stdout.clone())
        };

        // Drop-if-busy: producer skips a hop while a chunk is in flight.
        let pending = Arc::new(AtomicBool::new(false));
        // Cooperative producer shutdown signal (separate from stop_flag so
        // consumer can stop the producer if it exits unexpectedly).
        let producer_stop = Arc::new(AtomicBool::new(false));

        let p_stop_outer = stop_flag.clone();
        let p_stop_inner = producer_stop.clone();
        let p_full       = full_audio.clone();
        let p_stdin      = stdin_lock.clone();
        let p_pending    = pending.clone();
        let producer = tokio::spawn(async move {
            // First partial within ~300 ms: require ~0.3 s of audio.
            let min_frames = (sample_rate as f32 * 0.3) as usize;
            let mut tick = tokio::time::interval(
                std::time::Duration::from_millis(hop_ms as u64),
            );
            tick.set_missed_tick_behavior(
                tokio::time::MissedTickBehavior::Skip,
            );
            let mut seq: u64 = 0;
            loop {
                if p_stop_outer.load(Ordering::SeqCst)
                    || p_stop_inner.load(Ordering::SeqCst) { return; }
                tick.tick().await;
                if p_stop_outer.load(Ordering::SeqCst)
                    || p_stop_inner.load(Ordering::SeqCst) { return; }
                if p_pending.load(Ordering::SeqCst) { continue; }

                // Cumulative audio: clone the full growing buffer.
                let interleaved: Vec<f32> = match p_full.lock() {
                    Ok(g) => g.clone(),
                    Err(e) => {
                        eprintln!("[voice-prompt] full lock poisoned: {e}");
                        return;
                    }
                };
                if interleaved.len() < min_frames * channels as usize { continue; }

                // Downmix to mono for the daemon.
                let mono: Vec<f32> = if channels == 1 {
                    interleaved
                } else {
                    let ch = channels as usize;
                    let frames = interleaved.len() / ch;
                    let mut m = Vec::with_capacity(frames);
                    for f in 0..frames {
                        let mut acc = 0.0f32;
                        for c in 0..ch { acc += interleaved[f * ch + c]; }
                        m.push(acc / ch as f32);
                    }
                    m
                };

                seq = seq.wrapping_add(1);
                let wav_path = std::env::temp_dir()
                    .join(format!("voice-prompt-live-{seq}.wav"));
                if let Err(e) = audio_in::write_wav(&wav_path, &mono, sample_rate, 1) {
                    eprintln!("[voice-prompt] live WAV write: {e}");
                    continue;
                }

                let req = serde_json::json!({
                    "cmd": "stream_chunk",
                    "wav": wav_path.to_string_lossy(),
                    "seq": seq,
                }).to_string() + "\n";

                p_pending.store(true, Ordering::SeqCst);

                let mut g = p_stdin.lock().await;
                use tokio::io::AsyncWriteExt;
                if let Err(e) = g.write_all(req.as_bytes()).await {
                    eprintln!("[voice-prompt] stream_chunk write: {e}");
                    return;
                }
                let _ = g.flush().await;
            }
        });

        // Consumer: read events until Idle (after stream_stop) or EOF.
        let mut had_eof = false;
        loop {
            let event_opt = {
                let mut g = stdout_lock.lock().await;
                let line = match g.next_line().await {
                    Ok(Some(l)) => l,
                    Ok(None) => { had_eof = true; break; }
                    Err(e) => {
                        eprintln!("[voice-prompt] stream read: {e}");
                        break;
                    }
                };
                drop(g);
                serde_json::from_str::<serde_json::Value>(&line).ok().map(|v| (v, line))
            };
            let Some((v, _line)) = event_opt else { continue };

            // Clear pending on every event from the daemon (partial / error / status).
            pending.store(false, Ordering::SeqCst);

            let event = if let Some(ev) = v.get("event").and_then(|x| x.as_str()) {
                match ev {
                    "partial" => StreamEvent::Partial {
                        seq: v["seq"].as_u64().unwrap_or(0),
                        text: v["text"].as_str().unwrap_or("").to_string(),
                        duration_ms: v["duration_ms"].as_u64().unwrap_or(0),
                    },
                    "final" => StreamEvent::Final {
                        text: v["text"].as_str().unwrap_or("").to_string(),
                    },
                    other => StreamEvent::Error(format!("unknown event {other}")),
                }
            } else if let Some(st) = v.get("status").and_then(|x| x.as_str()) {
                match st {
                    "streaming" => StreamEvent::Started,
                    "idle" => StreamEvent::Idle,
                    "error" => StreamEvent::Error(
                        v["message"].as_str().unwrap_or("").to_string(),
                    ),
                    _ => StreamEvent::Error(format!("unknown status {st}")),
                }
            } else { continue };

            match event {
                StreamEvent::Started => {}
                StreamEvent::Idle => break,
                StreamEvent::Final { .. } => {}
                StreamEvent::Error(m) => eprintln!("[voice-prompt] daemon error: {m}"),
                StreamEvent::Partial { seq: s, text, duration_ms } => {
                    // Drop late partials post-release.
                    if release_gate.load(Ordering::SeqCst) { continue; }
                    let text = text.trim().to_string();
                    if text.is_empty() { continue; }
                    eprintln!(
                        "[voice-prompt] partial seq={s} ({duration_ms} ms): {text:?}"
                    );
                    let _ = app.emit(
                        "partial-transcript",
                        serde_json::json!({"text": text, "seq": s}),
                    );

                    if overlay_mode {
                        let _ = app.emit(
                            "show-overlay",
                            serde_json::json!({"text": text}),
                        );
                        if let Some(w) = app.get_webview_window("overlay") {
                            let _ = w.show();
                        }
                    } else {
                        let mut prev = last_partial.lock().await;
                        let prev_str = prev.clone();
                        match injection::rolling_inject(
                            &prev_str, &text, win_id.as_deref(),
                        ) {
                            Ok(()) => {
                                *prev = text.clone();
                                preview_len.store(
                                    text.chars().count(),
                                    Ordering::SeqCst,
                                );
                            }
                            Err(e) => eprintln!(
                                "[voice-prompt] rolling_inject: {e}"
                            ),
                        }
                    }
                }
            }
        }

        // Cooperative shutdown — signal producer and await it. No abort.
        producer_stop.store(true, Ordering::SeqCst);
        let _ = producer.await;

        if had_eof {
            *tiny_daemon.lock().await = None;
        }
        eprintln!("[voice-prompt] streaming pipeline exited");
    })
}

fn hide_overlay(app: &AppHandle) {
    let _ = app.emit("hide-overlay", serde_json::json!({}));
    if let Some(w) = app.get_webview_window("overlay") { let _ = w.hide(); }
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
