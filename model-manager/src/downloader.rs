use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use futures_util::StreamExt;
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub id: String,
    pub url: String,
    pub bytes: u64,
    pub total: Option<u64>,
    pub speed_bps: f64,
}

pub async fn download_to_path(
    id: &str,
    url: &str,
    dest: &PathBuf,
    cancel: CancellationToken,
    mut on_progress: impl FnMut(DownloadProgress),
) -> Result<()> {
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let resp = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .with_context(|| format!("HTTP GET failed: {url}"))?
        .error_for_status()?;

    let total = resp.content_length();
    let mut stream = resp.bytes_stream();
    let mut file = tokio::fs::File::create(dest).await?;
    let mut bytes: u64 = 0;
    let started = Instant::now();
    let mut last_emit = Instant::now();

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                drop(file);
                let _ = tokio::fs::remove_file(dest).await;
                anyhow::bail!("cancelled");
            }
            chunk = stream.next() => {
                match chunk {
                    Some(Ok(b)) => {
                        file.write_all(&b).await?;
                        bytes += b.len() as u64;
                        if last_emit.elapsed().as_millis() >= 50 {
                            let elapsed = started.elapsed().as_secs_f64().max(1e-3);
                            on_progress(DownloadProgress {
                                id: id.to_string(),
                                url: url.to_string(),
                                bytes,
                                total,
                                speed_bps: bytes as f64 / elapsed,
                            });
                            last_emit = Instant::now();
                        }
                    }
                    Some(Err(e)) => return Err(e.into()),
                    None => break,
                }
            }
        }
    }

    file.flush().await?;
    Ok(())
}
