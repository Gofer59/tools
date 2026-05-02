use crate::AppState;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn start_overlay(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut child_guard = state.overlay_child.lock().await;

    // Check if already running.
    if let Some(child) = child_guard.as_mut() {
        match child.try_wait() {
            Ok(None) => {
                // Still running.
                return Ok(());
            }
            _ => {
                // Exited or error — clear it.
                *child_guard = None;
            }
        }
    }

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let child = std::process::Command::new(&exe)
        .arg("--daemon")
        .spawn()
        .map_err(|e| e.to_string())?;

    *child_guard = Some(child);
    let _ = app.emit("overlay-state", serde_json::json!({"running": true}));
    Ok(())
}

#[tauri::command]
pub async fn stop_overlay(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut child_guard = state.overlay_child.lock().await;
    if let Some(mut child) = child_guard.take() {
        let _ = child.kill();
    }
    let _ = app.emit("overlay-state", serde_json::json!({"running": false}));
    Ok(())
}

#[tauri::command]
pub async fn is_overlay_running(state: State<'_, AppState>) -> Result<bool, String> {
    let mut child_guard = state.overlay_child.lock().await;
    if let Some(child) = child_guard.as_mut() {
        match child.try_wait() {
            Ok(None) => return Ok(true),
            _ => {
                *child_guard = None;
            }
        }
    }
    Ok(false)
}
