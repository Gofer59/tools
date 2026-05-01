use serde::Serialize;

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Serialize, Clone)]
pub struct HotkeyTriggered {
    pub tool: String,
    pub state: String,
}

#[allow(dead_code)]
#[derive(Serialize, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub duration_ms: u64,
}

#[allow(dead_code)]
#[derive(Serialize, Clone)]
pub struct DaemonReady {
    pub model: String,
}
