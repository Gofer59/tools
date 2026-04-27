use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

/// A chunk of raw 16-bit signed LE PCM audio from the daemon.
#[derive(Debug, Clone)]
pub struct PcmChunk {
    pub id: String,
    pub sample_rate: u32,
    pub samples: Vec<i16>,
}

pub struct DaemonHandle {
    pub child: Child,
    stdin: Mutex<ChildStdin>,
    reader: Mutex<BufReader<ChildStdout>>,
}

pub async fn spawn(python_bin: &str, script: &PathBuf, model_dir: &PathBuf) -> Result<DaemonHandle> {
    let mut child = Command::new(python_bin)
        .arg(script)
        .arg(model_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("spawning {python_bin} {script:?}"))?;

    let stdin = child.stdin.take().context("daemon stdin")?;
    let stdout = child.stdout.take().context("daemon stdout")?;
    let mut reader = BufReader::new(stdout);

    let mut first_line = String::new();
    reader.read_line(&mut first_line).await?;
    let v: Value = serde_json::from_str(first_line.trim())
        .with_context(|| format!("first line not JSON: {first_line}"))?;
    if v["status"] != "ready" {
        anyhow::bail!("daemon not ready: {first_line}");
    }

    Ok(DaemonHandle {
        child,
        stdin: Mutex::new(stdin),
        reader: Mutex::new(reader),
    })
}

/// Send a speak request and collect all PCM chunks until "done" or "error".
pub async fn speak(
    h: &DaemonHandle,
    text: &str,
    voice: &str,
    speed: f32,
    noise_scale: f32,
    noise_w_scale: f32,
    id: &str,
) -> Result<Vec<PcmChunk>> {
    let req = serde_json::json!({
        "cmd": "speak",
        "text": text,
        "voice": voice,
        "speed": speed,
        "noise_scale": noise_scale,
        "noise_w_scale": noise_w_scale,
        "id": id,
    })
    .to_string()
        + "\n";

    {
        let mut stdin = h.stdin.lock().await;
        stdin.write_all(req.as_bytes()).await?;
        stdin.flush().await?;
    }

    let mut reader = h.reader.lock().await;
    let mut chunks = Vec::new();

    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            anyhow::bail!("daemon EOF during speak");
        }
        let v: Value = serde_json::from_str(line.trim())
            .with_context(|| format!("response not JSON: {line}"))?;

        // PCM chunk header — arrives as its own line (one or more per "speaking" status)
        if let Some(n_bytes) = v.get("audio_pcm_bytes").and_then(|x| x.as_u64()) {
            let n_bytes = n_bytes as usize;
            let sample_rate = v["sample_rate"].as_u64().unwrap_or(22050) as u32;
            let chunk_id = v["id"].as_str().unwrap_or(id).to_string();

            let mut pcm_bytes = vec![0u8; n_bytes];
            // read_exact through the BufReader so its internal buffer stays consistent
            reader.read_exact(&mut pcm_bytes).await?;

            let samples: Vec<i16> = pcm_bytes
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]))
                .collect();

            chunks.push(PcmChunk { id: chunk_id, sample_rate, samples });
            continue;
        }

        let status = v["status"].as_str().unwrap_or("");
        match status {
            "speaking" => { /* informational; PCM chunk headers follow as separate lines */ }
            "done" => break,
            "error" => anyhow::bail!("daemon error: {}", v["message"]),
            _ => {}
        }
    }

    Ok(chunks)
}

pub async fn stop(h: &DaemonHandle, id: &str) -> Result<()> {
    let mut stdin = h.stdin.lock().await;
    let req = serde_json::json!({"cmd": "stop", "id": id}).to_string() + "\n";
    stdin.write_all(req.as_bytes()).await?;
    stdin.flush().await?;
    Ok(())
}

pub async fn quit(h: &mut DaemonHandle) {
    {
        if let Ok(mut stdin) = h.stdin.try_lock() {
            let _ = stdin.write_all(b"{\"cmd\":\"quit\"}\n").await;
            let _ = stdin.flush().await;
        }
    }
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), h.child.wait()).await;
    let _ = h.child.kill().await;
}
