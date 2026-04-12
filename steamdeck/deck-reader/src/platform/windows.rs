// Windows backend.
//
// Subtask 1.4 — implemented: capture_region (xcap), clipboard r/w (arboard),
//                             type_text stub (always errors — use clipboard).
// Subtask 1.5 — implemented:  select_region (egui fullscreen overlay).
// Subtask 1.6 — implemented:  spawn/kill TTS fallback (Child::kill; sounddevice
//                             has no subprocess tree so Job Object is phase-2 only).

use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

use anyhow::{anyhow, Context, Result};

use super::{Region, Selection};

pub fn backend_description() -> &'static str {
    "Windows (egui/xcap/arboard)"
}

/// `%APPDATA%\deck-reader` on Windows.
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("deck-reader")
}

/// `%LOCALAPPDATA%\deck-reader` on Windows.
pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("deck-reader")
}

/// No XDG_RUNTIME_DIR on Windows — falls back through cache_dir to temp_dir.
#[allow(dead_code)]
pub fn runtime_dir() -> PathBuf {
    dirs::runtime_dir()
        .or_else(dirs::cache_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("deck-reader")
}

pub fn tts_socket_path() -> PathBuf {
    // Windows has no Unix socket; phase-2 daemon uses TCP with an ephemeral port.
    // Return an empty path so callers that stringify it get something harmless.
    PathBuf::new()
}

// ─────────────────────────────────────────────────────────────────────────────
// Region selector — egui fullscreen overlay
// ─────────────────────────────────────────────────────────────────────────────

/// Transient egui app: covers the full virtual screen with a dark transparent
/// tint, lets the user drag a rubber-band rectangle, then closes itself and
/// sends the selected `Region` (or `None` on Esc) through the channel.
struct RegionSelector {
    result_tx:    std::sync::mpsc::SyncSender<Option<Region>>,
    /// Top-left of the virtual screen in physical pixels.
    screen_x:     i32,
    screen_y:     i32,
    drag_start:   Option<egui::Pos2>,
    drag_current: Option<egui::Pos2>,
    /// True once a result has been sent — we just wait for Close to fire.
    sent:         bool,
}

impl RegionSelector {
    fn new(
        tx: std::sync::mpsc::SyncSender<Option<Region>>,
        screen_x: i32,
        screen_y: i32,
    ) -> Self {
        Self {
            result_tx: tx,
            screen_x,
            screen_y,
            drag_start: None,
            drag_current: None,
            sent: false,
        }
    }
}

impl eframe::App for RegionSelector {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.sent {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // Esc → cancel.
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            let _ = self.result_tx.try_send(None);
            self.sent = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // Track drag via raw pointer input.
        let (pressed, released, pos) = ctx.input(|i| {
            (
                i.pointer.primary_pressed(),
                i.pointer.primary_released(),
                i.pointer.interact_pos(),
            )
        });

        if pressed {
            self.drag_start = pos;
            self.drag_current = pos;
        } else if self.drag_start.is_some() {
            if let Some(p) = pos {
                self.drag_current = Some(p);
            }
        }

        if released {
            if let (Some(start), Some(end)) = (self.drag_start, self.drag_current) {
                // Convert egui logical coords → physical pixels → virtual screen coords.
                let ppp = ctx.pixels_per_point();
                let rect = egui::Rect::from_two_pos(start, end);
                let region = Region {
                    x: (rect.min.x * ppp) as i32 + self.screen_x,
                    y: (rect.min.y * ppp) as i32 + self.screen_y,
                    w: ((rect.width()  * ppp) as i32).max(1),
                    h: ((rect.height() * ppp) as i32).max(1),
                };
                let _ = self.result_tx.try_send(Some(region));
                self.sent = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }

        // Paint overlay.
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(20, 20, 20)))
            .show(ctx, |ui| {
                let painter = ui.painter();

                painter.text(
                    egui::pos2(ui.max_rect().center().x, 28.0),
                    egui::Align2::CENTER_CENTER,
                    "Drag to select a region -- Esc to cancel",
                    egui::FontId::proportional(18.0),
                    egui::Color32::WHITE,
                );

                if let (Some(start), Some(end)) = (self.drag_start, self.drag_current) {
                    let r = egui::Rect::from_two_pos(start, end);
                    painter.rect_filled(r, 0.0, egui::Color32::from_white_alpha(30));
                    painter.rect_stroke(
                        r,
                        0.0,
                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                        egui::StrokeKind::Outside,
                    );
                }
            });

        ctx.request_repaint();
    }
}

/// Open a fullscreen transparent overlay and let the user drag a rectangle.
/// Returns the selected region in physical screen coordinates, or an error
/// if the user presses Esc or if eframe itself fails.
pub fn select_region() -> Result<Region> {
    let monitors = xcap::Monitor::all().context("Failed to enumerate monitors")?;
    if monitors.is_empty() {
        anyhow::bail!("No monitors found");
    }

    // Virtual-screen bounding box (spans all monitors).
    // xcap 0.8 monitor accessors return Result — unwrap with defaults.
    let virt_x = monitors.iter().filter_map(|m| m.x().ok()).min().unwrap_or(0);
    let virt_y = monitors.iter().filter_map(|m| m.y().ok()).min().unwrap_or(0);
    let virt_right  = monitors.iter().filter_map(|m| Some(m.x().ok()? + m.width().ok()? as i32)).max().unwrap_or(1920);
    let virt_bottom = monitors.iter().filter_map(|m| Some(m.y().ok()? + m.height().ok()? as i32)).max().unwrap_or(1080);
    let virt_w = (virt_right  - virt_x) as f32;
    let virt_h = (virt_bottom - virt_y) as f32;

    let (result_tx, result_rx) = std::sync::mpsc::sync_channel::<Option<Region>>(1);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("deck-reader: select region")
            .with_position([virt_x as f32, virt_y as f32])
            .with_inner_size([virt_w, virt_h])
            .with_decorations(false)
            .with_always_on_top(),
            // Note: with_transparent(true) causes a GPU hang / black screen on
            // Windows (DWM transparent swapchain init fails silently).
            // The overlay uses an opaque dark background instead.
        ..Default::default()
    };

    eframe::run_native(
        "deck-reader-select",
        options,
        Box::new(move |_cc| Ok(Box::new(RegionSelector::new(result_tx, virt_x, virt_y)))),
    )
    .map_err(|e| anyhow!("Region selector error: {e}"))?;

    match result_rx.try_recv() {
        Ok(Some(region)) => Ok(region),
        _ => anyhow::bail!("Region selection cancelled"),
    }
}

pub fn capture_region(region: &Region, output_path: &Path) -> Result<()> {
    let monitors = xcap::Monitor::all().context("Failed to enumerate monitors")?;

    // Find the monitor whose bounding box contains the region's top-left corner.
    // xcap 0.8 accessors return Result — use ok() defaults for the geometry check.
    let mon = monitors
        .iter()
        .find(|m| {
            let (mx, my) = (m.x().unwrap_or(0), m.y().unwrap_or(0));
            let (mw, mh) = (m.width().unwrap_or(0) as i32, m.height().unwrap_or(0) as i32);
            region.x >= mx && region.x < mx + mw && region.y >= my && region.y < my + mh
        })
        .ok_or_else(|| {
            anyhow!(
                "No monitor found containing region origin ({}, {}). \
                 Multi-monitor spans are not supported in the MVP.",
                region.x,
                region.y
            )
        })?;

    let image = mon.capture_image().context("Failed to capture monitor image")?;

    // Convert virtual-screen coords to monitor-local coords.
    let local_x = (region.x - mon.x().unwrap_or(0)) as u32;
    let local_y = (region.y - mon.y().unwrap_or(0)) as u32;

    let cropped = image::imageops::crop_imm(
        &image,
        local_x,
        local_y,
        region.w as u32,
        region.h as u32,
    )
    .to_image();

    cropped
        .save(output_path)
        .with_context(|| format!("Failed to save captured image to {:?}", output_path))?;

    Ok(())
}

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    arboard::Clipboard::new()
        .context("Failed to open clipboard")?
        .set_text(text.to_owned())
        .context("Failed to write to clipboard")?;
    Ok(())
}

pub fn type_text(_text: &str) -> Result<()> {
    Err(anyhow!(
        "delivery_mode = \"type\"/\"both\" is not supported on Windows (use \"clipboard\")"
    ))
}

pub fn read_clipboard() -> Result<String> {
    read_selection(Selection::Clipboard)
}

pub fn read_selection(_selection: Selection) -> Result<String> {
    // Windows has no PRIMARY selection concept — both variants read from the clipboard.
    arboard::Clipboard::new()
        .context("Failed to open clipboard")?
        .get_text()
        .context("Failed to read clipboard text")
}

/// Spawn the Python TTS script in a hidden console window.
///
/// On Windows the TTS audio is played inline via `sounddevice` (no child
/// processes), so a simple `Child::kill()` is sufficient to stop playback.
/// A Windows Job Object would be cleaner for true subprocess trees but is
/// deferred to phase 2.
///
/// The `_wrapper` parameter is unused on Windows — paths are derived from the
/// known data directory layout set by `install.ps1`.
pub fn spawn_tts_fallback(
    text: &str,
    voice: &str,
    speed: f32,
    _wrapper: &Path,
) -> Result<Child> {
    use std::os::windows::process::CommandExt;

    let data = super::data_dir();
    let python = data.join("venv").join("Scripts").join("python.exe");
    let script = data.join("python").join("tts_speak.py");
    // Pass the models directory explicitly so tts_speak.py finds the Piper
    // models even when XDG_DATA_HOME is not set (it never is on Windows).
    let models_dir = data.join("models");

    // CREATE_NO_WINDOW: suppress the console window that would otherwise flash.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    Command::new(&python)
        .arg(&script)
        .arg(text)
        .arg(voice)
        .arg(speed.to_string())
        .env("PIPER_MODELS_DIR", &models_dir)
        .stderr(Stdio::piped()) // captured so errors reach poll_tts_done
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .with_context(|| {
            format!(
                "Failed to launch TTS script at {:?}. Did you run install.ps1?",
                script
            )
        })
}

pub fn kill_tts_fallback(child: &mut Child) {
    if let Ok(Some(_)) = child.try_wait() {
        return; // Already exited.
    }
    println!("[deck-reader] Stopping TTS (fallback)…");
    let _ = child.kill();
    let _ = child.wait();
}
