use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn default_hotkey_quick() -> String { "F9".into() }
fn default_hotkey_select() -> String { "F10".into() }
fn default_hotkey_stop() -> String { "F11".into() }
fn default_ocr_language() -> String { "eng".into() }
fn default_delivery_mode() -> String { "clipboard".into() }
fn default_tts_voice() -> String { "en_US-lessac-medium".into() }
fn default_tts_speed() -> f32 { 1.0 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_hotkey_quick")]
    pub hotkey_quick_capture: String,
    #[serde(default = "default_hotkey_select")]
    pub hotkey_select_region: String,
    #[serde(default = "default_hotkey_stop")]
    pub hotkey_stop_tts: String,
    #[serde(default = "default_ocr_language")]
    pub ocr_language: String,
    /// "clipboard" | "type" | "both"
    #[serde(default = "default_delivery_mode")]
    pub delivery_mode: String,
    #[serde(default = "default_tts_voice")]
    pub tts_voice: String,
    #[serde(default = "default_tts_speed")]
    pub tts_speed: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey_quick_capture: default_hotkey_quick(),
            hotkey_select_region: default_hotkey_select(),
            hotkey_stop_tts: default_hotkey_stop(),
            ocr_language: default_ocr_language(),
            delivery_mode: default_delivery_mode(),
            tts_voice: default_tts_voice(),
            tts_speed: default_tts_speed(),
        }
    }
}

fn config_path(dir: &Path) -> PathBuf {
    dir.join("config.json")
}

pub fn load_or_default(dir: &Path) -> Config {
    let p = config_path(dir);
    if let Ok(b) = std::fs::read(&p) {
        if let Ok(c) = serde_json::from_slice::<Config>(&b) {
            return c;
        }
    }
    Config::default()
}

pub fn save(dir: &Path, cfg: &Config) -> Result<()> {
    let p = config_path(dir);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(p, serde_json::to_vec_pretty(cfg)?)?;
    Ok(())
}
