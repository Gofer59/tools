use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn wrapper_script(app: &AppHandle) -> PathBuf {
    // 1. Environment override
    if let Ok(p) = std::env::var("SCREEN_OCR_WRAPPER") {
        let p = PathBuf::from(p);
        if p.exists() {
            return p;
        }
    }

    // 2. Tauri resource directory (bundled in production)
    if let Ok(res_dir) = app.path().resource_dir() {
        let candidate = res_dir.join("ocr_extract_wrapper.sh");
        if candidate.exists() {
            return candidate;
        }
    }

    // 3. app_local_data_dir (installed via install.sh)
    if let Ok(data_dir) = app.path().app_local_data_dir() {
        let candidate = data_dir.join("ocr_extract_wrapper.sh");
        if candidate.exists() {
            return candidate;
        }
    }

    // 4. ~/.local/share/screen-ocr/
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let candidate = home
        .join(".local")
        .join("share")
        .join("screen-ocr")
        .join("ocr_extract_wrapper.sh");
    if candidate.exists() {
        return candidate;
    }

    // 5. Source tree fallback (dev mode)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("..")
        .join("python")
        .join("ocr_extract_wrapper.sh")
}
