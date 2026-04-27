use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn daemon_script(app: &AppHandle) -> PathBuf {
    if let Ok(p) = std::env::var("VOICE_PROMPT_DAEMON") {
        return PathBuf::from(p);
    }
    if let Ok(r) = app.path().resource_dir() {
        let p = r.join("whisper_daemon.py");
        if p.exists() {
            return p;
        }
    }
    if let Ok(r) = app.path().app_local_data_dir() {
        let p = r.join("whisper_daemon.py");
        if p.exists() {
            return p;
        }
    }
    // Dev fallback: look in the worktree python/ directory
    PathBuf::from(
        std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.to_path_buf()))
            .unwrap_or_default()
            .join("../../../python/whisper_daemon.py"),
    )
}
