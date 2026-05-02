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

    let mut changed_fields: Vec<(String, Value)> = vec![];
    for (k, v) in partial_obj {
        obj.insert(k.clone(), v.clone());
        changed_fields.push((k.clone(), v.clone()));
    }

    let new_cfg: Config = serde_json::from_value(cfg_json).map_err(|e| e.to_string())?;
    *cfg_w = new_cfg.clone();
    drop(cfg_w);

    let dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    save(&dir, &new_cfg).map_err(|e| e.to_string())?;

    for (field, value) in &changed_fields {
        match field.as_str() {
            "hotkey_quick_capture" => {
                let mut h = state.hotkey_quick.lock().await;
                crate::hotkey::unregister(&app, &mut h);
                *h = crate::hotkey::register(
                    &app,
                    value.as_str().unwrap_or("F9"),
                    "quick",
                )
                .map_err(|e| e.to_string())?;
            }
            "hotkey_select_region" => {
                let mut h = state.hotkey_select.lock().await;
                crate::hotkey::unregister(&app, &mut h);
                *h = crate::hotkey::register(
                    &app,
                    value.as_str().unwrap_or("F10"),
                    "select",
                )
                .map_err(|e| e.to_string())?;
            }
            "hotkey_stop_tts" => {
                let mut h = state.hotkey_stop.lock().await;
                crate::hotkey::unregister(&app, &mut h);
                *h = crate::hotkey::register(
                    &app,
                    value.as_str().unwrap_or("F11"),
                    "stop",
                )
                .map_err(|e| e.to_string())?;
            }
            _ => {}
        }
        let _ = app.emit(
            "config-applied",
            serde_json::json!({"field": field, "value": value}),
        );
    }

    Ok(())
}
