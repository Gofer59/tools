use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::paths;
use tauri::AppHandle;

pub fn extract(app: &AppHandle, image_path: &Path, language: &str) -> Result<String> {
    let wrapper = paths::wrapper_script(app);
    eprintln!("[screen-ocr] Running OCR (lang={language})…");

    let output = Command::new(&wrapper)
        .arg(image_path)
        .arg(language)
        .output()
        .with_context(|| {
            format!(
                "Failed to run OCR script at {:?}. Did you run install.sh?",
                wrapper
            )
        })?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        eprint!("{}", stderr);
    }

    if !output.status.success() {
        anyhow::bail!("OCR script failed:\n{}", stderr);
    }

    let text = String::from_utf8(output.stdout)
        .context("OCR output was not valid UTF-8")?
        .trim()
        .to_owned();

    Ok(text)
}
