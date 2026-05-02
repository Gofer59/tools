use crate::config::{save, Config};
use crate::AppState;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    Ok(state.config.read().await.clone())
}

#[tauri::command]
pub async fn update_config(
    app: AppHandle,
    state: State<'_, AppState>,
    partial: Value,
) -> Result<(), String> {
    let mut cfg_w = state.config.write().await;
    let mut cfg_json = serde_json::to_value(&*cfg_w).map_err(|e| e.to_string())?;
    let obj = cfg_json.as_object_mut().ok_or("config not an object")?;
    let partial_obj = partial.as_object().ok_or("partial not an object")?;

    let mut changed_hotkey = false;
    for (k, v) in partial_obj {
        // Skip null values and empty strings (partial-update semantics).
        if v.is_null() || v.as_str().is_some_and(|s| s.is_empty()) {
            continue;
        }
        let is_hotkey = k == "region_select_hotkey" || k == "toggle_on_top_hotkey";
        if is_hotkey {
            changed_hotkey = true;
        }
        obj.insert(k.clone(), v.clone());
    }

    let new_cfg: Config = serde_json::from_value(cfg_json).map_err(|e| e.to_string())?;
    *cfg_w = new_cfg.clone();
    drop(cfg_w);

    let dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    save(&new_cfg, &dir).map_err(|e| e.to_string())?;

    // If a hotkey field changed, restart the overlay so it picks up the new binding.
    if changed_hotkey {
        let mut child_guard = state.overlay_child.lock().await;
        if let Some(child) = child_guard.as_mut() {
            if matches!(child.try_wait(), Ok(None)) {
                let _ = child.kill();
                let exe = std::env::current_exe().map_err(|e| e.to_string())?;
                match std::process::Command::new(&exe).arg("--daemon").spawn() {
                    Ok(new_child) => {
                        *child_guard = Some(new_child);
                        let _ = app.emit("overlay-state", serde_json::json!({"running": true}));
                    }
                    Err(e) => {
                        *child_guard = None;
                        eprintln!("[threshold-filter] overlay restart failed: {e}");
                    }
                }
            }
        }
    }

    let _ = app.emit("config-applied", serde_json::json!({"partial": partial_obj}));
    Ok(())
}
