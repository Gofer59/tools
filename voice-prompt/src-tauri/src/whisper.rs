use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use anyhow::{Context, Result};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex as TokioMutex;

pub type StdinLock  = Arc<TokioMutex<ChildStdin>>;
pub type StdoutLock = Arc<TokioMutex<Lines<BufReader<ChildStdout>>>>;

pub struct DaemonHandle {
    pub child: Child,
    pub stdin: StdinLock,
    pub stdout: StdoutLock,
    pub model: String,
}

pub async fn spawn(
    python_bin: &str,
    script: &PathBuf,
    model: &str,
    compute_type: &str,
    model_dir: &PathBuf,
    device: &str,
) -> Result<DaemonHandle> {
    let mut child = Command::new(python_bin)
        .arg(script)
        .arg(model)
        .arg(compute_type)
        .arg(model_dir)
        .arg(device)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("spawning {python_bin} {script:?}"))?;

    let stdin  = child.stdin.take().context("daemon stdin")?;
    let stdout = child.stdout.take().context("daemon stdout")?;
    let mut lines = BufReader::new(stdout).lines();

    let first = lines.next_line().await?.unwrap_or_default();
    let v: Value = serde_json::from_str(&first)
        .with_context(|| format!("first line not JSON: {first}"))?;
    if v["status"] != "ready" {
        anyhow::bail!("daemon failed to become ready: {first}");
    }

    Ok(DaemonHandle {
        child,
        stdin:  Arc::new(TokioMutex::new(stdin)),
        stdout: Arc::new(TokioMutex::new(lines)),
        model:  model.to_string(),
    })
}

/// Single-shot transcribe. Atomic: takes both locks.
pub async fn transcribe(
    h: &DaemonHandle,
    wav: &Path,
    language: &str,
    vad: bool,
) -> Result<(String, u64)> {
    let req = json!({
        "cmd": "transcribe",
        "wav": wav.to_string_lossy(),
        "language": language,
        "vad": vad,
    }).to_string() + "\n";

    // Lock order: stdin first, then stdout.
    let mut stdin_g = h.stdin.lock().await;
    stdin_g.write_all(req.as_bytes()).await?;
    stdin_g.flush().await?;
    drop(stdin_g);

    let mut stdout_g = h.stdout.lock().await;
    let line = stdout_g.next_line().await?.context("daemon EOF during transcribe")?;
    drop(stdout_g);

    let v: Value = serde_json::from_str(&line)
        .with_context(|| format!("response not JSON: {line}"))?;
    if v["status"] != "ok" {
        anyhow::bail!("daemon error: {}", v["message"]);
    }
    let text = v["text"].as_str().unwrap_or("").to_string();
    let dt_ms = v["duration_ms"].as_u64().unwrap_or(0);
    Ok((text, dt_ms))
}

pub async fn quit(h: &mut DaemonHandle) {
    {
        let mut g = h.stdin.lock().await;
        let _ = g.write_all(b"{\"cmd\":\"quit\"}\n").await;
        let _ = g.flush().await;
    }
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        h.child.wait(),
    ).await;
    let _ = h.child.kill().await;
}
