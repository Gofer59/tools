use anyhow::{Context, Result};
use std::io::Write as _;
use std::process::{Command, Stdio};

pub fn inject(text: &str) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }

    // Write the transcription to the X11 clipboard.
    // Typing character-by-character (xdotool type) loses spaces at high speed;
    // a single Ctrl+V paste is reliable regardless of text length.
    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()
        .context("xclip not found — install xclip")?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())?;
    }
    child.wait().context("xclip wait")?;

    // Brief pause for modifiers to be fully released, then paste.
    std::thread::sleep(std::time::Duration::from_millis(80));
    Command::new("xdotool")
        .args(["key", "--clearmodifiers", "ctrl+v"])
        .status()
        .context("xdotool key ctrl+v")?;

    Ok(())
}
