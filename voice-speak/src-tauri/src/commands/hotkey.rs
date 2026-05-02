use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, State};

use crate::AppState;

#[tauri::command]
pub async fn test_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if state.test_hotkey_armed.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let _ = app.emit("hotkey-test-armed", serde_json::json!({}));
    let armed = state.test_hotkey_armed.clone();
    crate::hotkey::spawn_capture_thread(app, armed);
    Ok(())
}

#[tauri::command]
pub async fn preview_voice(
    state: State<'_, AppState>,
    text: String,
) -> Result<(), String> {
    let cfg = state.config.read().await.clone();
    let chunks = {
        let daemon = state.daemon_ctrl.lock().await;
        crate::piper::speak(
            &daemon,
            &text,
            &cfg.voice,
            cfg.speed,
            cfg.noise_scale,
            cfg.noise_w_scale,
            "preview",
        )
        .await
    };
    match chunks {
        Ok(pcm) => state
            .audio_tx
            .send(crate::AudioCmd::Play(pcm))
            .await
            .map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}
