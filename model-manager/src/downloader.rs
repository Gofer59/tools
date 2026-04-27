use serde::Serialize;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;
use anyhow::Result;

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub id: String,
    pub url: String,
    pub bytes: u64,
    pub total: Option<u64>,
    pub speed_bps: f64,
}

pub async fn download_to_path(
    _id: &str,
    _url: &str,
    _dest: &PathBuf,
    _cancel: CancellationToken,
    _on_progress: impl FnMut(DownloadProgress),
) -> Result<()> {
    Ok(())
}
