use anyhow::{Context, Result};
use std::process::Command;

// ── Platform dispatch ─────────────────────────────────────────────────────────

/// Inject text at the cursor. On Wayland, requires `wtype`. On Windows, uses enigo.
pub fn inject(text: &str, window_id: Option<&str>) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }
    dispatch_inject(text, window_id)
}

// ── Linux / macOS implementation ──────────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
fn dispatch_inject(text: &str, window_id: Option<&str>) -> Result<()> {
    match detect_display_server() {
        DisplayServer::Wayland => inject_wayland(text),
        DisplayServer::X11 => inject_x11(text, window_id),
    }
}

#[cfg(not(target_os = "windows"))]
#[derive(Debug, Clone, Copy)]
enum DisplayServer { X11, Wayland }

#[cfg(not(target_os = "windows"))]
fn detect_display_server() -> DisplayServer {
    match std::env::var("XDG_SESSION_TYPE").as_deref() {
        Ok("wayland") => DisplayServer::Wayland,
        Ok("x11")    => DisplayServer::X11,
        _ => {
            if std::env::var("WAYLAND_DISPLAY").is_ok() {
                DisplayServer::Wayland
            } else {
                DisplayServer::X11
            }
        }
    }
}

// ── X11 ──────────────────────────────────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
fn inject_x11(text: &str, window_id: Option<&str>) -> Result<()> {
    if text.is_empty() { return Ok(()); }

    let mut cmd = Command::new("xdotool");
    if let Some(id) = window_id {
        cmd.args(["windowfocus", "--sync", id]);
    }
    cmd.args(["type", "--clearmodifiers", "--delay", "8", "--", text]);
    let st = cmd.status().context("xdotool type")?;
    if !st.success() {
        anyhow::bail!("xdotool type failed: {st}");
    }
    Ok(())
}

// ── Wayland ───────────────────────────────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
fn inject_wayland(text: &str) -> Result<()> {
    let st = Command::new("wtype")
        .arg("--")
        .arg(text)
        .status()
        .context(
            "wtype not found — Wayland injection requires `wtype` installed \
             (apt install wtype / pacman -S wtype).",
        )?;
    if !st.success() {
        anyhow::bail!("wtype failed: {st}");
    }
    Ok(())
}

// ── Windows ───────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn dispatch_inject(text: &str, _window_id: Option<&str>) -> Result<()> {
    inject_windows(text)
}

#[cfg(target_os = "windows")]
fn inject_windows(text: &str) -> Result<()> {
    use enigo::{Enigo, Keyboard, Settings};
    let mut enigo = Enigo::new(&Settings::default()).context("enigo init")?;
    enigo.text(text).context("enigo text")?;
    Ok(())
}
