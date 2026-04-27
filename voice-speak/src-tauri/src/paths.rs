use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn daemon_script(app: &AppHandle) -> PathBuf {
    if let Ok(p) = std::env::var("VOICE_SPEAK_DAEMON") {
        return PathBuf::from(p);
    }
    if let Ok(r) = app.path().resource_dir() {
        let p = r.join("piper_daemon.py");
        if p.exists() {
            return p;
        }
    }
    if let Ok(r) = app.path().app_local_data_dir() {
        let p = r.join("piper_daemon.py");
        if p.exists() {
            return p;
        }
    }
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../python/piper_daemon.py"
    ))
}
