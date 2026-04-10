// Linux backend for the deck-reader platform layer.
//
// Auto-detects X11 vs Wayland at first use and caches the result.
// All IO primitives shell out to native Linux tools:
//   X11:     slop  / maim / xclip    / xdotool
//   Wayland: slurp / grim / wl-paste / ydotool

use std::{
    io::Write,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::OnceLock,
};

use anyhow::{Context, Result};

use super::{Region, Selection};

// ─────────────────────────────────────────────────────────────────────────────
// Display server detection (cached)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum DisplayServer {
    X11,
    Wayland,
}

fn detect_display_server() -> DisplayServer {
    if let Ok(session) = std::env::var("XDG_SESSION_TYPE") {
        if session.eq_ignore_ascii_case("wayland") {
            return DisplayServer::Wayland;
        }
    }
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        return DisplayServer::Wayland;
    }
    DisplayServer::X11
}

/// Cached display-server detection — first call decides, subsequent calls are free.
fn current_display() -> DisplayServer {
    static CACHE: OnceLock<DisplayServer> = OnceLock::new();
    *CACHE.get_or_init(detect_display_server)
}

/// Short description of the active backend, for the startup banner.
pub fn backend_description() -> &'static str {
    match current_display() {
        DisplayServer::X11 => "X11 (slop/maim/xclip)",
        DisplayServer::Wayland => "Wayland (slurp/grim/wl-paste)",
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Path helpers (via `dirs` crate — cross-platform XDG / KnownFolder resolution)
// ─────────────────────────────────────────────────────────────────────────────

/// `~/.config/deck-reader` on Linux (or $XDG_CONFIG_HOME/deck-reader).
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("deck-reader")
}

/// `~/.local/share/deck-reader` on Linux (or $XDG_DATA_HOME/deck-reader).
pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("deck-reader")
}

/// `$XDG_RUNTIME_DIR/deck-reader` on Linux (or cache/temp fallback).
/// Currently unused; available for future code that needs a runtime-scoped dir.
#[allow(dead_code)]
pub fn runtime_dir() -> PathBuf {
    dirs::runtime_dir()
        .or_else(dirs::cache_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("deck-reader")
}

/// Unix socket path for the TTS daemon.
/// Prefers $XDG_RUNTIME_DIR, falls back to /tmp with uid suffix for multi-user safety.
pub fn tts_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("deck-reader-tts.sock")
    } else {
        let uid = unsafe { libc::getuid() };
        PathBuf::from(format!("/tmp/deck-reader-tts-{uid}.sock"))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Screen capture — interactive region selection
//   X11:     slop --format="%w %h %x %y"
//   Wayland: slurp  (outputs "X,Y WxH")
// ─────────────────────────────────────────────────────────────────────────────

pub fn select_region() -> Result<Region> {
    match current_display() {
        DisplayServer::X11 => {
            let output = Command::new("slop")
                .args(["--format=%w %h %x %y"])
                .output()
                .context("Failed to run slop. Install: sudo pacman -S slop")?;

            if !output.status.success() {
                anyhow::bail!("slop exited non-zero (user cancelled selection?)");
            }

            let text = String::from_utf8(output.stdout)
                .context("slop output was not valid UTF-8")?
                .trim()
                .to_owned();

            // Parse "W H X Y"
            let parts: Vec<i32> = text
                .split_whitespace()
                .map(|s| s.parse::<i32>())
                .collect::<std::result::Result<Vec<_>, _>>()
                .with_context(|| format!("Failed to parse slop geometry: {:?}", text))?;

            if parts.len() != 4 {
                anyhow::bail!("Unexpected slop output (expected 4 values): {:?}", text);
            }

            Ok(Region { w: parts[0], h: parts[1], x: parts[2], y: parts[3] })
        }
        DisplayServer::Wayland => {
            let output = Command::new("slurp")
                .output()
                .context("Failed to run slurp. Install: sudo pacman -S slurp")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.is_empty() {
                    anyhow::bail!("slurp exited non-zero (user cancelled selection?)");
                } else {
                    anyhow::bail!("slurp failed: {}", stderr.trim());
                }
            }

            // slurp outputs: "X,Y WxH"
            let text = String::from_utf8(output.stdout)
                .context("slurp output was not valid UTF-8")?
                .trim()
                .to_owned();

            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.len() != 2 {
                anyhow::bail!("Unexpected slurp output: {:?}", text);
            }

            let parse_pair = |s: &str, sep: char, label: &str| -> Result<(i32, i32)> {
                let v: Vec<i32> = s
                    .split(sep)
                    .map(|n| n.parse::<i32>())
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .with_context(|| format!("Failed to parse slurp {} {:?}", label, s))?;
                if v.len() != 2 {
                    anyhow::bail!("Unexpected slurp {} format: {:?}", label, s);
                }
                Ok((v[0], v[1]))
            };

            let (x, y) = parse_pair(parts[0], ',', "X,Y")?;
            let (w, h) = parse_pair(parts[1], 'x', "WxH")?;

            Ok(Region { x, y, w, h })
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Screen capture — known geometry
//   X11:     maim -g WxH+X+Y output.png
//   Wayland: grim -g "X,Y WxH" output.png
// ─────────────────────────────────────────────────────────────────────────────

pub fn capture_region(region: &Region, output_path: &Path) -> Result<()> {
    let path_str = output_path.to_string_lossy();

    match current_display() {
        DisplayServer::X11 => {
            let geom = format!("{}x{}+{}+{}", region.w, region.h, region.x, region.y);
            let status = Command::new("maim")
                .args(["-g", &geom, &*path_str])
                .status()
                .context("Failed to run maim. Install: sudo pacman -S maim")?;
            if !status.success() {
                anyhow::bail!("maim exited non-zero");
            }
        }
        DisplayServer::Wayland => {
            let geom = format!("{},{} {}x{}", region.x, region.y, region.w, region.h);
            let status = Command::new("grim")
                .args(["-g", &geom, &*path_str])
                .status()
                .context("Failed to run grim. Install: sudo pacman -S grim")?;
            if !status.success() {
                anyhow::bail!("grim exited non-zero");
            }
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Clipboard write
//   X11:     xclip -selection clipboard
//   Wayland: wl-copy
// ─────────────────────────────────────────────────────────────────────────────

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut child = match current_display() {
        DisplayServer::X11 => Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(Stdio::piped())
            .spawn()
            .context("Failed to run xclip. Install: sudo pacman -S xclip")?,
        DisplayServer::Wayland => Command::new("wl-copy")
            .stdin(Stdio::piped())
            .spawn()
            .context("Failed to run wl-copy. Install: sudo pacman -S wl-clipboard")?,
    };

    // Write text then close stdin (drop) so the tool sees EOF and exits.
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .context("Failed to write to clipboard stdin")?;
    }

    let status = child.wait().context("Failed to wait for clipboard tool")?;
    if !status.success() {
        anyhow::bail!("clipboard copy exited non-zero");
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Text injection
//   X11:     xdotool type
//   Wayland: ydotool type (requires ydotoold daemon)
// ─────────────────────────────────────────────────────────────────────────────

pub fn type_text(text: &str) -> Result<()> {
    match current_display() {
        DisplayServer::X11 => {
            let status = Command::new("xdotool")
                .args(["type", "--clearmodifiers", "--delay", "0", "--", text])
                .status()
                .context("Failed to run xdotool. Install: sudo pacman -S xdotool")?;
            if !status.success() {
                anyhow::bail!("xdotool exited non-zero");
            }
        }
        DisplayServer::Wayland => {
            let status = Command::new("ydotool")
                .args(["type", "--", text])
                .status()
                .context(
                    "Failed to run ydotool. Install: sudo pacman -S ydotool\n\
                     Note: ydotoold daemon must be running.",
                )?;
            if !status.success() {
                anyhow::bail!("ydotool exited non-zero");
            }
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Clipboard read
//   X11:     xclip -selection primary/clipboard -o
//   Wayland: wl-paste [--primary] --no-newline
// ─────────────────────────────────────────────────────────────────────────────

/// Read PRIMARY selection (highlighted text), fall back to CLIPBOARD.
pub fn read_clipboard() -> Result<String> {
    let text = read_selection(Selection::Primary)?;
    if !text.is_empty() {
        return Ok(text);
    }
    read_selection(Selection::Clipboard)
}

pub fn read_selection(selection: Selection) -> Result<String> {
    let output = match current_display() {
        DisplayServer::X11 => {
            let sel_str = match selection {
                Selection::Primary => "primary",
                Selection::Clipboard => "clipboard",
            };
            Command::new("xclip")
                .args(["-selection", sel_str, "-o"])
                .output()
                .context("Failed to run xclip. Install: sudo pacman -S xclip")?
        }
        DisplayServer::Wayland => {
            let mut cmd = Command::new("wl-paste");
            if selection == Selection::Primary {
                cmd.arg("--primary");
            }
            cmd.arg("--no-newline");
            cmd.output()
                .context("Failed to run wl-paste. Install: sudo pacman -S wl-clipboard")?
        }
    };

    // xclip / wl-paste exit non-zero when the selection is empty.
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            eprintln!("[deck-reader] clipboard stderr: {}", stderr.trim());
        }
        return Ok(String::new());
    }

    Ok(String::from_utf8(output.stdout)
        .context("Clipboard content was not valid UTF-8")?
        .trim()
        .to_owned())
}

// ─────────────────────────────────────────────────────────────────────────────
// TTS fallback (cold-start subprocess, used when daemon is unavailable)
// ─────────────────────────────────────────────────────────────────────────────

/// Spawn tts_speak_wrapper.sh as a detached subprocess in its own process group,
/// so `kill_tts_fallback` can nuke the whole tree (wrapper + python + paplay).
pub fn spawn_tts_fallback(text: &str, voice: &str, speed: f32, wrapper: &Path) -> Result<Child> {
    // SAFETY: setsid() is async-signal-safe and has no preconditions.
    let child = unsafe {
        Command::new(wrapper)
            .arg(text)
            .arg(voice)
            .arg(speed.to_string())
            .pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            })
            .spawn()
            .with_context(|| {
                format!(
                    "Failed to run TTS wrapper at {:?}. Did you run install.sh?",
                    wrapper
                )
            })?
    };

    Ok(child)
}

/// Kill a fallback TTS subprocess and all its children (wrapper → python → paplay).
pub fn kill_tts_fallback(child: &mut Child) {
    if let Ok(Some(_)) = child.try_wait() {
        return;
    }
    println!("[deck-reader] Stopping TTS (fallback)…");
    let pid = child.id() as i32;
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
    }
    let _ = child.wait();
}
