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
            "push_to_talk_key" => {
                let mut h = state.hotkey_ctrl.lock().await;
                crate::hotkey::unregister(&app, &mut h);
                *h = crate::hotkey::register(&app, value.as_str().unwrap_or("Ctrl+Alt+Space"))
                    .map_err(|e| e.to_string())?;
            }

            // Large daemon restart triggers.
            "whisper_model" | "compute_type" | "python_bin" => {
                restart_large_daemon(&app, &state, &new_cfg, &dir).await?;
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

async fn restart_large_daemon(
    app: &AppHandle,
    state: &AppState,
    cfg: &Config,
    dir: &std::path::Path,
) -> Result<(), String> {
    let mut d = state.daemon_ctrl.lock().await;
    crate::whisper::quit(&mut d).await;
    let script = crate::paths::daemon_script(app);
    let model_dir = dir.join("models");
    *d = crate::whisper::spawn(
        &cfg.python_bin,
        &script,
        &cfg.whisper_model,
        &cfg.compute_type,
        &model_dir,
        "cpu",
    )
    .await
    .map_err(|e| e.to_string())?;
    let _ = app.emit("daemon-ready", serde_json::json!({"model": cfg.whisper_model}));
    Ok(())
}
