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

/// Delete `n` Unicode characters before the cursor (simulates Backspace ×n).
/// Used to erase the tiny-model preview before injecting the large-model final text.
///
/// Race window: there is a small gap between the last backspace and the final
/// inject during which user keystrokes could corrupt cursor position. Acceptable
/// for typical large-model latency (1-3 s swap windows); documented in TESTING.md.
pub fn delete_chars(n: usize, window_id: Option<&str>) -> Result<()> {
    if n == 0 {
        return Ok(());
    }
    dispatch_delete(n, window_id)
}

/// Release any modifier keys that xdotool --clearmodifiers may have left in a
/// logically-pressed state.  Call this once after ALL injection is complete and
/// AFTER the user has physically released their PTT keys.  On X11, `xdotool type
/// --clearmodifiers` saves the current modifier mask, clears it, types, then
/// re-asserts it — which can leave Alt "doubly pressed" relative to the physical
/// state, causing the next left-click to be interpreted as Alt+click (= window
/// drag in most window managers).  Sending explicit keyup events here resets the
/// counter to zero.
///
/// No-op on Wayland (wtype has no modifier persistence) and Windows (enigo resets
/// state per call).
pub fn release_modifiers() {
    #[cfg(not(target_os = "windows"))]
    {
        if std::env::var("XDG_SESSION_TYPE").as_deref() != Ok("wayland")
            && std::env::var("WAYLAND_DISPLAY").is_err()
        {
            let _ = std::process::Command::new("xdotool")
                .args(["keyup", "ctrl", "alt", "super", "shift"])
                .status();
        }
    }
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
fn dispatch_delete(n: usize, window_id: Option<&str>) -> Result<()> {
    match detect_display_server() {
        DisplayServer::Wayland => delete_chars_wayland(n),
        DisplayServer::X11 => delete_chars_x11(n, window_id),
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
    if let Some(id) = window_id {
        let _ = Command::new("xdotool")
            .args(["windowfocus", "--sync", id])
            .status();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // XTestFakeKeyEvent looks like real keyboard input; apps cannot reject it
    // and avoids X11 clipboard async timing issues.
    let st = Command::new("xdotool")
        .args(["type", "--clearmodifiers", "--delay", "12", "--", text])
        .status()
        .context("xdotool type")?;
    if !st.success() {
        anyhow::bail!("xdotool type failed: {st}");
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn delete_chars_x11(n: usize, window_id: Option<&str>) -> Result<()> {
    if let Some(id) = window_id {
        let _ = Command::new("xdotool")
            .args(["windowfocus", "--sync", id])
            .status();
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    let st = Command::new("xdotool")
        .args([
            "key", "--clearmodifiers",
            "--repeat", &n.to_string(),
            "--delay", "0",
            "BackSpace",
        ])
        .status()
        .context("xdotool key BackSpace")?;
    if !st.success() {
        anyhow::bail!("xdotool BackSpace failed: {st}");
    }
    eprintln!("[voice-prompt] xdotool BackSpace ×{n}");
    Ok(())
}

// ── Wayland ───────────────────────────────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
fn inject_wayland(text: &str) -> Result<()> {
    // wtype must be installed (apt install wtype / pacman -S wtype).
    let st = Command::new("wtype")
        .arg("--")
        .arg(text)
        .status()
        .context(
            "wtype not found — Wayland injection requires `wtype` installed \
             (apt install wtype / pacman -S wtype). \
             Switch to overlay mode or install wtype.",
        )?;
    if !st.success() {
        anyhow::bail!("wtype failed: {st}");
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn delete_chars_wayland(n: usize) -> Result<()> {
    for _ in 0..n {
        let st = Command::new("wtype")
            .args(["-k", "BackSpace"])
            .status()
            .context("wtype BackSpace")?;
        if !st.success() {
            anyhow::bail!("wtype BackSpace failed: {st}");
        }
    }
    eprintln!("[voice-prompt] wtype BackSpace ×{n}");
    Ok(())
}

// ── Windows ───────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn dispatch_inject(text: &str, _window_id: Option<&str>) -> Result<()> {
    inject_windows(text)
}

#[cfg(target_os = "windows")]
fn dispatch_delete(n: usize, _window_id: Option<&str>) -> Result<()> {
    delete_chars_windows(n)
}

#[cfg(target_os = "windows")]
fn inject_windows(text: &str) -> Result<()> {
    use enigo::{Enigo, Keyboard, Settings};
    let mut enigo = Enigo::new(&Settings::default()).context("enigo init")?;
    enigo.text(text).context("enigo text")?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn delete_chars_windows(n: usize) -> Result<()> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    let mut enigo = Enigo::new(&Settings::default()).context("enigo init")?;
    for _ in 0..n {
        enigo.key(Key::Backspace, Direction::Click).context("enigo Backspace")?;
    }
    Ok(())
}

// ── Rolling-diff inject (live-preview path) ─────────────────────────────────

/// Apply the minimal keystroke sequence to transform `prev` into `next` at the
/// cursor: backspace the divergent char-suffix, then type the new suffix.
/// Operates on Unicode scalar chars (matches `delete_chars`).
pub fn rolling_inject(prev: &str, next: &str, window_id: Option<&str>) -> Result<()> {
    if prev == next { return Ok(()); }
    let p: Vec<char> = prev.chars().collect();
    let n: Vec<char> = next.chars().collect();
    let mut common = 0usize;
    while common < p.len() && common < n.len() && p[common] == n[common] {
        common += 1;
    }
    let to_delete = p.len() - common;
    let suffix: String = n[common..].iter().collect();
    if to_delete > 0 { delete_chars(to_delete, window_id)?; }
    if !suffix.is_empty() { inject(&suffix, window_id)?; }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    /// Preview length must use Unicode scalar count, not byte count.
    /// "à bientôt" = 9 Unicode scalars but 11 bytes in UTF-8.
    #[test]
    fn test_preview_len_unicode() {
        let preview = "à bientôt";
        assert_eq!(preview.chars().count(), 9);
        assert_ne!(preview.len(), 9); // bytes != scalars
    }

    /// delete_chars(0) must be a no-op.
    #[test]
    fn test_delete_zero_is_noop() {
        assert!(super::delete_chars(0, None).is_ok());
    }

    #[test]
    fn test_rolling_diff_pure_extension() {
        let prev = "hello";
        let next = "hello world";
        let p: Vec<char> = prev.chars().collect();
        let n: Vec<char> = next.chars().collect();
        let mut c = 0;
        while c < p.len() && c < n.len() && p[c] == n[c] { c += 1; }
        assert_eq!(c, 5);
        let suffix: String = n[c..].iter().collect();
        assert_eq!(suffix, " world");
    }

    #[test]
    fn test_rolling_diff_unicode_revision() {
        let prev = "à bientôt";
        let next = "à bientôt!";
        let p: Vec<char> = prev.chars().collect();
        let n: Vec<char> = next.chars().collect();
        let mut c = 0;
        while c < p.len() && c < n.len() && p[c] == n[c] { c += 1; }
        assert_eq!(c, 9);
        let suffix: String = n[c..].iter().collect();
        assert_eq!(suffix, "!");
    }
}
