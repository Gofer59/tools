use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub hotkey: String,
    pub voice: String,
    pub speed: f32,
    pub noise_scale: f32,
    pub noise_w_scale: f32,
    pub python_bin: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Alt+V".into(),
            voice: "en_US-lessac-medium".into(),
            speed: 1.0,
            noise_scale: 0.667,
            noise_w_scale: 0.8,
            python_bin: "python3".into(),
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
