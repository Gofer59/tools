use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::catalog::ModelKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserModelEntry {
    pub id: String,
    pub kind: ModelKind,
    pub display_name: String,
    pub language: String,
    pub onnx_path: PathBuf,
    pub config_path: Option<PathBuf>,
    pub size_bytes: u64,
    pub sha256: String,
    pub added_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModel {
    pub id: String,
    pub kind: ModelKind,
    pub display_name: String,
    pub language: String,
    pub size_bytes: u64,
    pub source: LocalSource,
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocalSource {
    Catalog,
    User,
}

pub fn user_models_path(app_local_data: &Path) -> PathBuf {
    app_local_data.join("user_models.json")
}

pub fn load_user_models(app_local_data: &Path) -> Result<Vec<UserModelEntry>> {
    let p = user_models_path(app_local_data);
    if !p.exists() {
        return Ok(vec![]);
    }
    let bytes = std::fs::read(&p)?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn save_user_models(app_local_data: &Path, list: &[UserModelEntry]) -> Result<()> {
    let p = user_models_path(app_local_data);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(p, serde_json::to_vec_pretty(list)?)?;
    Ok(())
}
