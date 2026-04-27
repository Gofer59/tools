use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelKind {
    Whisper,
    Piper,
}

/// Catalog entry using `'static` references — Serialize only (no Deserialize).
#[derive(Debug, Clone, Serialize)]
pub struct ModelEntry {
    pub id: &'static str,
    pub kind: ModelKind,
    pub display_name: &'static str,
    pub language: &'static str,
    pub size_bytes: u64,
    pub license: &'static str,
    pub urls: &'static [&'static str],
    pub sha256: Option<&'static str>,
    pub multilingual: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhisperModel {
    pub entry: ModelEntry,
}

#[derive(Debug, Clone, Serialize)]
pub struct PiperVoice {
    pub entry: ModelEntry,
}

pub static WHISPER_MODELS: &[ModelEntry] = &[];
pub static PIPER_VOICES: &[ModelEntry] = &[];

pub fn whisper_by_id(id: &str) -> Option<&'static ModelEntry> {
    WHISPER_MODELS.iter().find(|m| m.id == id)
}

pub fn piper_by_id(id: &str) -> Option<&'static ModelEntry> {
    PIPER_VOICES.iter().find(|m| m.id == id)
}

pub fn entries_for(kind: ModelKind) -> &'static [ModelEntry] {
    match kind {
        ModelKind::Whisper => WHISPER_MODELS,
        ModelKind::Piper => PIPER_VOICES,
    }
}
