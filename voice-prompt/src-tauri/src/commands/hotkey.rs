use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, State};

use crate::AppState;

#[tauri::command]
pub async fn test_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.test_hotkey_armed.store(true, Ordering::SeqCst);
    let _ = app.emit("hotkey-test-armed", serde_json::json!({}));
    Ok(())
}
