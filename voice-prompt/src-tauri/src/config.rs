use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub push_to_talk_key: String,
    pub whisper_model: String,
    pub language: String,
    pub vad_filter: bool,
    pub python_bin: String,
    pub compute_type: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            push_to_talk_key: "Ctrl+Alt+Space".into(),
            whisper_model: "small".into(),
            language: "en".into(),
            vad_filter: true,
            python_bin: "python3".into(),
            compute_type: "int8".into(),
        }
    }
}

pub fn config_path(app_local_data: &Path) -> PathBuf {
    app_local_data.join("config.json")
}

pub fn load_or_default(app_local_data: &Path) -> Config {
    let p = config_path(app_local_data);
    if let Ok(b) = std::fs::read(&p) {
        if let Ok(c) = serde_json::from_slice::<Config>(&b) {
            return c;
        }
    }
    Config::default()
}

pub fn save(app_local_data: &Path, cfg: &Config) -> Result<()> {
    let p = config_path(app_local_data);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(p, serde_json::to_vec_pretty(cfg)?)?;
    Ok(())
}

pub type SharedConfig = Arc<RwLock<Config>>;

pub fn shared(c: Config) -> SharedConfig {
    Arc::new(RwLock::new(c))
}
