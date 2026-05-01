use crate::AppState;
use crate::events::{DownloadComplete, DownloadError, DownloadProgressPayload};
use model_manager::{
    LocalModel, ModelKind, UserModelEntry, WHISPER_MODELS,
    load_user_models, save_user_models,
};
use model_manager::user_models::LocalSource;
use model_manager::catalog::ModelEntry;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

#[tauri::command]
pub async fn list_catalog_models() -> Result<Vec<ModelEntry>, String> {
    Ok(WHISPER_MODELS.to_vec())
}

#[tauri::command]
pub async fn list_local_models(app: AppHandle) -> Result<Vec<LocalModel>, String> {
    let dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let mut out = vec![];

    for m in WHISPER_MODELS {
        let mp = dir.join("models").join(m.id);
        if mp.exists() {
            out.push(LocalModel {
                id: m.id.into(),
                kind: m.kind,
                display_name: m.display_name.into(),
                language: m.language.into(),
                size_bytes: m.size_bytes,
                source: LocalSource::Catalog,
                paths: vec![mp],
            });
        }
    }

    let users = load_user_models(&dir).map_err(|e| e.to_string())?;
    for u in users {
        out.push(LocalModel {
            id: u.id,
            kind: u.kind,
            display_name: u.display_name,
            language: u.language,
            size_bytes: u.size_bytes,
            source: LocalSource::User,
            paths: std::iter::once(u.onnx_path)
                .chain(u.config_path)
                .collect(),
        });
    }

    Ok(out)
}

#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let entry = WHISPER_MODELS
        .iter()
        .find(|m| m.id == id)
        .ok_or("unknown model id")?
        .clone();

    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("models")
        .join(entry.id);

    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;

    let token = CancellationToken::new();
    state
        .download_map
        .lock()
        .await
        .insert(entry.id.into(), token.clone());

    let app2 = app.clone();
    let id2 = entry.id.to_string();

    tokio::spawn(async move {
        for url in entry.urls {
            let fname = url.rsplit('/').next().unwrap_or("file");
            let dest = dir.join(fname);
            let app3 = app2.clone();
            let id3 = id2.clone();

            let res = model_manager::download_to_path(
                &id2,
                url,
                &dest,
                token.clone(),
                |p| {
                    let _ = app3.emit(
                        "download-progress",
                        DownloadProgressPayload {
                            id: id3.clone(),
                            bytes: p.bytes,
                            total: p.total,
                            speed_bps: p.speed_bps,
                        },
                    );
                },
            )
            .await;

            if let Err(e) = res {
                let _ = app2.emit(
                    "download-error",
                    DownloadError {
                        id: id2.clone(),
                        message: e.to_string(),
                    },
                );
                return;
            }

            let sha = model_manager::verify::sha256_file(&dest).unwrap_or_default();
            let _ = app2.emit(
                "download-complete",
                DownloadComplete {
                    id: id2.clone(),
                    sha256: sha,
                    path: dest.to_string_lossy().to_string(),
                },
            );
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn cancel_download(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    if let Some(tok) = state.download_map.lock().await.remove(&id) {
        tok.cancel();
    }
    Ok(())
}

#[tauri::command]
pub async fn add_custom_model(app: AppHandle, path: String) -> Result<LocalModel, String> {
    let p = PathBuf::from(&path);
    if p.extension().map(|e| e != "onnx").unwrap_or(true) {
        return Err(".onnx file required".into());
    }
    let cfg_path = {
        let mut c = p.clone();
        let name = c.file_name().unwrap().to_string_lossy().to_string() + ".json";
        c.set_file_name(name);
        c
    };
    if !cfg_path.exists() {
        return Err("missing sibling .onnx.json config file".into());
    }

    let dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dest_dir = dir.join("models").join("user");
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;

    let onnx_dest = dest_dir.join(p.file_name().unwrap());
    let cfg_dest = dest_dir.join(cfg_path.file_name().unwrap());
    std::fs::copy(&p, &onnx_dest).map_err(|e| e.to_string())?;
    std::fs::copy(&cfg_path, &cfg_dest).map_err(|e| e.to_string())?;

    let size = std::fs::metadata(&onnx_dest)
        .map_err(|e| e.to_string())?
        .len();
    let sha = model_manager::verify::sha256_file(&onnx_dest).map_err(|e| e.to_string())?;
    let id = onnx_dest
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let entry = UserModelEntry {
        id: id.clone(),
        kind: ModelKind::Whisper,
        display_name: id.clone(),
        language: "custom".into(),
        onnx_path: onnx_dest.clone(),
        config_path: Some(cfg_dest.clone()),
        size_bytes: size,
        sha256: sha,
        added_at_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    let mut list = load_user_models(&dir).map_err(|e| e.to_string())?;
    list.push(entry.clone());
    save_user_models(&dir, &list).map_err(|e| e.to_string())?;

    Ok(LocalModel {
        id: entry.id,
        kind: entry.kind,
        display_name: entry.display_name,
        language: entry.language,
        size_bytes: entry.size_bytes,
        source: LocalSource::User,
        paths: vec![onnx_dest, cfg_dest],
    })
}

#[tauri::command]
pub async fn delete_local_model(app: AppHandle, id: String) -> Result<(), String> {
    let dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let cat = dir.join("models").join(&id);
    if cat.exists() {
        let _ = std::fs::remove_dir_all(&cat);
        return Ok(());
    }
    let mut list = load_user_models(&dir).map_err(|e| e.to_string())?;
    list.retain(|e| {
        if e.id == id {
            let _ = std::fs::remove_file(&e.onnx_path);
            if let Some(c) = &e.config_path {
                let _ = std::fs::remove_file(c);
            }
            false
        } else {
            true
        }
    });
    save_user_models(&dir, &list).map_err(|e| e.to_string())?;
    Ok(())
}
