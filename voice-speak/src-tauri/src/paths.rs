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
    // User-visible install path: ~/.local/share/voice-speak/piper_daemon.py
    if let Ok(home) = app.path().home_dir() {
        let p = home.join(".local/share/voice-speak/piper_daemon.py");
        if p.exists() {
            return p;
        }
    }
    // Dev fallback: source tree python/ dir
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../python/piper_daemon.py"
    ))
}
