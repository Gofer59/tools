use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

const DEFAULT_CONFIG_TOML: &str = r#"[hotkeys]
# Key names: F8, F9, F10, MetaLeft+KeyQ, AltLeft+KeyU, etc.
# Raw keycodes from Steam Input: "191" or "Unknown(191)"
region_select   = "F10"
toggle_on_top   = "F8"

[display]
default_threshold = 128       # 0-255
invert            = false     # swap black/white
always_on_top     = true
panel_width       = 50.0      # left control panel width in pixels
"#;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    pub hotkeys: HotkeyConfig,
    pub display: DisplayConfig,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HotkeyConfig {
    pub region_select: String,
    #[serde(alias = "cycle_threshold")]
    pub toggle_on_top: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisplayConfig {
    #[serde(default = "default_threshold")]
    pub default_threshold: u8,
    #[serde(default)]
    pub invert: bool,
    #[serde(default = "default_true")]
    pub always_on_top: bool,
    #[serde(default = "default_panel_width")]
    pub panel_width: f32,
}

fn default_threshold() -> u8 {
    128
}
fn default_true() -> bool {
    true
}
fn default_panel_width() -> f32 {
    50.0
}

pub fn config_dir() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .unwrap_or_else(|_| format!("{}/.config", std::env::var("HOME").unwrap_or_default()));
    PathBuf::from(base).join("threshold-filter")
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .unwrap_or_else(|_| format!("{}/.local/share", std::env::var("HOME").unwrap_or_default()));
    PathBuf::from(base).join("threshold-filter")
}

pub fn load_config() -> Result<AppConfig> {
    let dir = config_dir();
    let path = dir.join("config.toml");

    if !path.exists() {
        fs::create_dir_all(&dir)
            .with_context(|| format!("Cannot create config directory {dir:?}"))?;
        fs::write(&path, DEFAULT_CONFIG_TOML)
            .with_context(|| format!("Cannot write default config to {path:?}"))?;
        println!("[threshold-filter] Created default config at {path:?}");
    }

    let text = fs::read_to_string(&path)
        .with_context(|| format!("Cannot read config file {path:?}"))?;

    toml::from_str(&text)
        .with_context(|| format!("Malformed config at {path:?} -- fix the TOML then restart"))
}
