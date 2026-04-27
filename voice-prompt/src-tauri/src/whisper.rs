use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{Context, Result};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

pub struct DaemonHandle {
    pub child: Child,
    pub stdin: ChildStdin,
    pub stdout_lines: Lines<BufReader<ChildStdout>>,
    pub model: String,
}

pub async fn spawn(
    python_bin: &str,
    script: &PathBuf,
    model: &str,
    compute_type: &str,
    model_dir: &PathBuf,
) -> Result<DaemonHandle> {
    let mut child = Command::new(python_bin)
        .arg(script)
        .arg(model)
        .arg(compute_type)
        .arg(model_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("spawning {python_bin} {script:?}"))?;

    let stdin = child.stdin.take().context("daemon stdin")?;
    let stdout = child.stdout.take().context("daemon stdout")?;
    let mut stdout_lines = BufReader::new(stdout).lines();

    let first_line = stdout_lines
        .next_line()
        .await?
        .unwrap_or_default();

    let v: Value = serde_json::from_str(&first_line)
        .with_context(|| format!("first line not JSON: {first_line}"))?;

    if v["status"] != "ready" {
        anyhow::bail!("daemon failed to become ready: {first_line}");
    }

    Ok(DaemonHandle {
        child,
        stdin,
        stdout_lines,
        model: model.to_string(),
    })
}

pub async fn transcribe(
    h: &mut DaemonHandle,
    wav: &PathBuf,
    language: &str,
    vad: bool,
) -> Result<(String, u64)> {
    let req = json!({
        "cmd": "transcribe",
        "wav": wav.to_string_lossy(),
        "language": language,
        "vad": vad,
    })
    .to_string()
        + "\n";

    h.stdin.write_all(req.as_bytes()).await?;
    h.stdin.flush().await?;

    let line = h
        .stdout_lines
        .next_line()
        .await?
        .context("daemon EOF during transcribe")?;

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
    let _ = h.stdin.write_all(b"{\"cmd\":\"quit\"}\n").await;
    let _ = h.stdin.flush().await;
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        h.child.wait(),
    )
    .await;
    let _ = h.child.kill().await;
}
