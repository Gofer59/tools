use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

fn default_tiny_model()    -> String { "tiny".into() }
fn default_preview_mode()  -> String { "inline-replace".into() }
fn default_cpu()           -> String { "cpu".into() }
fn default_window_seconds()-> f32    { 6.0 }
fn default_hop_ms()        -> u32    { 250 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub push_to_talk_key: String,
    pub whisper_model: String,
    #[serde(default = "default_tiny_model")]
    pub tiny_model: String,
    #[serde(default = "default_preview_mode")]
    pub preview_mode: String,
    #[serde(default = "default_cpu")]
    pub tiny_device: String,
    #[serde(default = "default_cpu")]
    pub large_device: String,
    pub language: String,
    pub vad_filter: bool,
    pub python_bin: String,
    pub compute_type: String,
    #[serde(default = "default_window_seconds")]
    pub live_window_seconds: f32,
    #[serde(default = "default_hop_ms")]
    pub live_hop_ms: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            push_to_talk_key: "Ctrl+Alt+Space".into(),
            whisper_model: "small".into(),
            tiny_model: "tiny".into(),
            preview_mode: "inline-replace".into(),
            tiny_device: "cpu".into(),
            large_device: "cpu".into(),
            language: "en".into(),
            vad_filter: true,
            python_bin: "python3".into(),
            compute_type: "int8".into(),
            live_window_seconds: default_window_seconds(),
            live_hop_ms: default_hop_ms(),
        }
    }
}

pub fn config_path(app_local_data: &Path) -> PathBuf {
    app_local_data.join("config.json")
}

pub fn load_or_default(app_local_data: &Path) -> Config {
    let p = config_path(app_local_data);
    if let Ok(b) = std::fs::read(&p) {
        if let Ok(c) = serde_json::from_slice::<Config>(&b) { return c; }
    }
    Config::default()
}

pub fn save(app_local_data: &Path, cfg: &Config) -> Result<()> {
    let p = config_path(app_local_data);
    if let Some(parent) = p.parent() { std::fs::create_dir_all(parent)?; }
    std::fs::write(p, serde_json::to_vec_pretty(cfg)?)?;
    Ok(())
}

#[allow(dead_code)]
pub type SharedConfig = Arc<RwLock<Config>>;

#[allow(dead_code)]
pub fn shared(c: Config) -> SharedConfig { Arc::new(RwLock::new(c)) }
