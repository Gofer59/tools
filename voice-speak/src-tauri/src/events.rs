use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct ConfigApplied {
    pub field: String,
    pub value: serde_json::Value,
}

#[derive(Serialize, Clone)]
pub struct DownloadProgressPayload {
    pub id: String,
    pub bytes: u64,
    pub total: Option<u64>,
    pub speed_bps: f64,
}

#[derive(Serialize, Clone)]
pub struct DownloadComplete {
    pub id: String,
    pub sha256: String,
    pub path: String,
}

#[derive(Serialize, Clone)]
pub struct DownloadError {
    pub id: String,
    pub message: String,
}

#[derive(Serialize, Clone)]
pub struct HotkeyTriggered {
    pub tool: String,
    pub state: String,
}

#[derive(Serialize, Clone)]
pub struct DaemonReady {
    pub ready: bool,
}

#[derive(Serialize, Clone)]
pub struct TtsState {
    pub state: String,           // "speaking" | "done" | "stopped" | "error"
    pub message: Option<String>,
}
