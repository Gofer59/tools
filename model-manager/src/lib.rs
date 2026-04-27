pub mod catalog;
pub mod downloader;
pub mod user_models;
pub mod verify;

pub use catalog::{ModelEntry, ModelKind, WhisperModel, PiperVoice, WHISPER_MODELS, PIPER_VOICES};
pub use downloader::{download_to_path, DownloadProgress};
pub use user_models::{UserModelEntry, load_user_models, save_user_models, LocalModel};
