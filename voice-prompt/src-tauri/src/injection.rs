use anyhow::{Context, Result};
use std::process::Command;

pub fn inject(text: &str) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }
    // xdotool with per-keystroke delay; without --delay spaces get dropped at
    // high typing speed in most desktop environments on X11.
    Command::new("xdotool")
        .args(["type", "--clearmodifiers", "--delay", "12", "--", text])
        .status()
        .context("xdotool type")?;
    Ok(())
}
