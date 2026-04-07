use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// A sub-region within a captured window (window-relative coordinates).
#[derive(Debug, Clone)]
pub struct WindowCrop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Result of a region selection — carries everything needed for capture + positioning.
#[derive(Debug, Clone)]
pub struct SelectionResult {
    /// Target window X11 ID (Some on X11, None on Wayland).
    pub window_id: Option<u32>,
    /// Crop within the target window (Some on X11, None on Wayland).
    pub crop: Option<WindowCrop>,
    /// Screen-absolute region (always set — used for window positioning + Wayland capture).
    pub screen_region: Region,
}

// ---------------------------------------------------------------------------
// Display server detection — same pattern as deck-reader
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayServer {
    X11,
    Wayland,
}

pub fn detect_display_server() -> DisplayServer {
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

// ---------------------------------------------------------------------------
// Region selection
//   X11:     xdotool selectwindow → xdotool getwindowgeometry → slop
//   Wayland: slurp  (outputs "X,Y WxH")
// ---------------------------------------------------------------------------

pub fn select_region(display: DisplayServer) -> Result<SelectionResult> {
    match display {
        DisplayServer::X11 => select_with_window_x11(),
        DisplayServer::Wayland => {
            let region = select_region_wayland()?;
            Ok(SelectionResult {
                window_id: None,
                crop: None,
                screen_region: region,
            })
        }
    }
}

/// X11 two-step selection: click target window, then draw region.
/// Same pattern as desktop variant's `do_selection_linux()`.
fn select_with_window_x11() -> Result<SelectionResult> {
    // Step 1: click on the target window
    eprintln!("[threshold-filter] Click on the window to capture...");
    let output = Command::new("xdotool")
        .arg("selectwindow")
        .output()
        .context("Failed to run xdotool selectwindow. Install: sudo pacman -S xdotool")?;
    if !output.status.success() {
        anyhow::bail!("Window selection cancelled");
    }
    let wid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let window_id: u32 = wid_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid window ID: {wid_str}"))?;
    eprintln!("[threshold-filter] Selected window ID: {window_id}");

    // Step 2: get window position
    let geo_output = Command::new("xdotool")
        .args(["getwindowgeometry", "--shell", &wid_str])
        .output()
        .context("Failed to get window geometry")?;
    let geo_text = String::from_utf8_lossy(&geo_output.stdout);
    let mut win_x: i32 = 0;
    let mut win_y: i32 = 0;
    for line in geo_text.lines() {
        if let Some(val) = line.strip_prefix("X=") {
            win_x = val.parse().unwrap_or(0);
        }
        if let Some(val) = line.strip_prefix("Y=") {
            win_y = val.parse().unwrap_or(0);
        }
    }
    eprintln!("[threshold-filter] Window position: ({win_x}, {win_y})");

    // Step 3: draw a sub-region with slop
    eprintln!("[threshold-filter] Draw a rectangle on the area you want...");
    let slop_output = Command::new("slop")
        .arg("--format=%w %h %x %y")
        .output()
        .context("Failed to run slop. Install: sudo pacman -S slop")?;
    if !slop_output.status.success() {
        anyhow::bail!("Region selection cancelled");
    }
    let text = String::from_utf8_lossy(&slop_output.stdout);
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() < 4 {
        anyhow::bail!("Unexpected slop output: {text}");
    }
    let slop_w: i32 = parts[0].parse().context("Failed to parse slop width")?;
    let slop_h: i32 = parts[1].parse().context("Failed to parse slop height")?;
    let slop_x: i32 = parts[2].parse().context("Failed to parse slop x")?;
    let slop_y: i32 = parts[3].parse().context("Failed to parse slop y")?;

    // Step 4: compute window-relative crop
    let crop_x = (slop_x - win_x).max(0) as u32;
    let crop_y = (slop_y - win_y).max(0) as u32;

    eprintln!(
        "[threshold-filter] Region: {}x{} at ({},{}) — crop ({},{}) in window",
        slop_w, slop_h, slop_x, slop_y, crop_x, crop_y
    );

    Ok(SelectionResult {
        window_id: Some(window_id),
        crop: Some(WindowCrop {
            x: crop_x,
            y: crop_y,
            width: slop_w as u32,
            height: slop_h as u32,
        }),
        screen_region: Region {
            x: slop_x,
            y: slop_y,
            w: slop_w,
            h: slop_h,
        },
    })
}

fn select_region_wayland() -> Result<Region> {
    eprintln!("[threshold-filter] Running slurp (Wayland)...");
    let output = Command::new("slurp")
        .output()
        .context("Failed to run slurp. Install: sudo pacman -S slurp")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("slurp failed (status {}): {}", output.status, stderr.trim());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let text = text.trim();

    // Parse "X,Y WxH"
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() != 2 {
        anyhow::bail!("Unexpected slurp output: {text:?}");
    }

    let pos: Vec<i32> = parts[0]
        .split(',')
        .map(|n| n.parse::<i32>())
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("Failed to parse slurp position: {:?}", parts[0]))?;

    let size: Vec<i32> = parts[1]
        .split('x')
        .map(|n| n.parse::<i32>())
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("Failed to parse slurp size: {:?}", parts[1]))?;

    if pos.len() != 2 || size.len() != 2 {
        anyhow::bail!("Unexpected slurp output format: {text:?}");
    }

    Ok(Region {
        x: pos[0],
        y: pos[1],
        w: size[0],
        h: size[1],
    })
}

// ---------------------------------------------------------------------------
// Screen capture
//   X11:     xcap (window by ID + crop) — avoids self-capture
//   Wayland: grim -g "X,Y WxH" output.png (fallback)
// ---------------------------------------------------------------------------

/// Capture a specific window by X11 ID, optionally cropping to a sub-region.
/// Returns RGBA pixel data + dimensions. Same as desktop variant.
pub fn capture_window(window_id: u32, crop: Option<&WindowCrop>) -> Result<(Vec<u8>, u32, u32)> {
    let windows = xcap::Window::all().context("failed to list windows")?;
    let window = windows
        .into_iter()
        .find(|w| w.id().unwrap_or(0) == window_id)
        .ok_or_else(|| anyhow::anyhow!("window {window_id} not found — was it closed?"))?;

    let image = window.capture_image().context("failed to capture window")?;

    if let Some(c) = crop {
        let (img_w, img_h) = image.dimensions();
        if c.x >= img_w || c.y >= img_h {
            anyhow::bail!("crop region outside window bounds");
        }
        let w = c.width.min(img_w - c.x).max(1);
        let h = c.height.min(img_h - c.y).max(1);
        let cropped = image.view(c.x, c.y, w, h).to_image();
        let (w, h) = cropped.dimensions();
        Ok((cropped.into_raw(), w, h))
    } else {
        let (w, h) = image.dimensions();
        Ok((image.into_raw(), w, h))
    }
}

/// Capture a screen region using grim (Wayland fallback).
pub fn capture_region(region: &Region) -> Result<(Vec<u8>, u32, u32)> {
    let tmp = NamedTempFile::with_suffix(".png")
        .context("Failed to create temp file for grim")?;
    let path = tmp.path().to_string_lossy().to_string();

    let geom = format!("{},{} {}x{}", region.x, region.y, region.w, region.h);
    let status = Command::new("grim")
        .args(["-g", &geom, &path])
        .status()
        .context("Failed to run grim. Install: sudo pacman -S grim")?;

    if !status.success() {
        anyhow::bail!("grim exited non-zero");
    }

    let img = image::open(tmp.path())
        .with_context(|| format!("Failed to decode grim output at {:?}", tmp.path()))?
        .into_rgba8();

    let (w, h) = img.dimensions();
    Ok((img.into_raw(), w, h))
}

// ---------------------------------------------------------------------------
// Region persistence
// ---------------------------------------------------------------------------

pub fn load_region(path: &Path) -> Result<Region> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("No saved region at {path:?} -- use F10 first"))?;
    serde_json::from_str(&contents).context("Failed to parse saved region JSON")
}

pub fn save_region(path: &Path, region: &Region) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Cannot create directory {parent:?}"))?;
    }
    let json = serde_json::to_string_pretty(region)?;
    fs::write(path, json).with_context(|| format!("Cannot write region to {path:?}"))
}
