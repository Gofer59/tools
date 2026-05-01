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

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum StreamEvent {
    Started,
    Partial { seq: u64, text: String, duration_ms: u64 },
    Final { text: String },
    Idle,
    Error(String),
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

/// Single-shot transcribe (used by the large daemon). Atomic: takes both locks.
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

/// Send `stream_start`. Awaits the `{"status":"streaming"}` ack.
pub async fn stream_start(
    h: &DaemonHandle,
    language: &str,
    vad: bool,
    sample_rate: u32,
    window_seconds: f32,
    hop_ms: u32,
) -> Result<()> {
    let req = json!({
        "cmd": "stream_start",
        "language": language,
        "vad": vad,
        "sample_rate": sample_rate,
        "window_seconds": window_seconds,
        "hop_ms": hop_ms,
    }).to_string() + "\n";

    {
        let mut g = h.stdin.lock().await;
        g.write_all(req.as_bytes()).await?;
        g.flush().await?;
    }
    let mut g = h.stdout.lock().await;
    let line = g.next_line().await?.context("daemon EOF on stream_start")?;
    let v: Value = serde_json::from_str(&line)
        .with_context(|| format!("response not JSON: {line}"))?;
    if v["status"] != "streaming" {
        anyhow::bail!("daemon refused stream_start: {line}");
    }
    Ok(())
}

/// Producer side: write a `stream_chunk` (stdin only — no read).
#[allow(dead_code)]
pub async fn stream_chunk(h: &DaemonHandle, wav: &Path, seq: u64) -> Result<()> {
    let req = json!({
        "cmd": "stream_chunk",
        "wav": wav.to_string_lossy(),
        "seq": seq,
    }).to_string() + "\n";
    let mut g = h.stdin.lock().await;
    g.write_all(req.as_bytes()).await?;
    g.flush().await?;
    Ok(())
}

/// `stream_stop` — write request, drain until `idle`, return final text.
#[allow(dead_code)]
pub async fn stream_stop(h: &DaemonHandle) -> Result<String> {
    {
        let mut g = h.stdin.lock().await;
        g.write_all(b"{\"cmd\":\"stream_stop\"}\n").await?;
        g.flush().await?;
    }
    let mut g = h.stdout.lock().await;
    let mut final_text = String::new();
    loop {
        let line = g.next_line().await?
            .context("daemon EOF before idle on stream_stop")?;
        let v: Value = serde_json::from_str(&line)
            .with_context(|| format!("stream_stop response not JSON: {line}"))?;
        if v.get("event").and_then(|x| x.as_str()) == Some("final") {
            final_text = v["text"].as_str().unwrap_or("").to_string();
        }
        if v.get("status").and_then(|x| x.as_str()) == Some("idle") {
            return Ok(final_text);
        }
    }
}

/// Consumer side: read one streamed event (stdout only — no write).
#[allow(dead_code)]
pub async fn next_stream_event(h: &DaemonHandle) -> Result<Option<StreamEvent>> {
    let mut g = h.stdout.lock().await;
    let line = match g.next_line().await? {
        Some(l) => l,
        None => return Ok(None),
    };
    drop(g);
    let v: Value = serde_json::from_str(&line)
        .with_context(|| format!("stream event not JSON: {line}"))?;
    if let Some(ev) = v.get("event").and_then(|x| x.as_str()) {
        return Ok(Some(match ev {
            "partial" => StreamEvent::Partial {
                seq: v["seq"].as_u64().unwrap_or(0),
                text: v["text"].as_str().unwrap_or("").to_string(),
                duration_ms: v["duration_ms"].as_u64().unwrap_or(0),
            },
            "final" => StreamEvent::Final {
                text: v["text"].as_str().unwrap_or("").to_string(),
            },
            other => StreamEvent::Error(format!("unknown event {other}")),
        }));
    }
    if let Some(st) = v.get("status").and_then(|x| x.as_str()) {
        return Ok(Some(match st {
            "streaming" => StreamEvent::Started,
            "idle" => StreamEvent::Idle,
            "error" => StreamEvent::Error(v["message"].as_str().unwrap_or("").to_string()),
            _ => StreamEvent::Error(format!("unknown status {st}")),
        }));
    }
    Ok(Some(StreamEvent::Error(format!("malformed daemon line: {line}"))))
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
