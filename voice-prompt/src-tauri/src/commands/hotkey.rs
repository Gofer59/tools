#[allow(unused_imports)]
use std::sync::Arc;
use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, State};

use crate::AppState;

#[tauri::command]
pub async fn test_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Guard: don't start a second capture if one is already running.
    if state.test_hotkey_armed.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let _ = app.emit("hotkey-test-armed", serde_json::json!({}));

    // Spawn a temporary rdev listener that captures the next key combination.
    // rdev::listen blocks forever, so we run it in a detached thread and let
    // it idle after capturing one combo.
    let armed = state.test_hotkey_armed.clone();
    crate::hotkey::spawn_capture_thread(app, armed);

    Ok(())
}
