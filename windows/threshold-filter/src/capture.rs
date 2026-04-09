use anyhow::{Context, Result};
use image::GenericImageView;

/// A sub-region within a captured window (window-relative coordinates)
#[derive(Debug, Clone)]
pub struct WindowCrop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Capture a specific window by ID, optionally cropping to a sub-region.
/// Returns RGBA pixel data + dimensions.
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
