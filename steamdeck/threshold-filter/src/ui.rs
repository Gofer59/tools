use eframe::egui;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::capture::{self, DisplayServer, Region, SelectionResult, WindowCrop};
use crate::processing;

/// Actions sent from the hotkey dispatcher to the UI.
pub enum HotkeyAction {
    ToggleOnTop,
}

pub struct ThresholdApp {
    // Controls
    threshold: u8,
    invert: bool,
    always_on_top: bool,
    panel_collapsed: bool,
    panel_width: f32,

    // Capture state
    display: DisplayServer,
    target_window_id: Option<u32>,
    target_crop: Option<WindowCrop>,
    region: Option<Region>,
    region_file: PathBuf,

    // Auto-capture
    target_fps: f32,
    last_capture_time: Instant,

    // Image state
    texture: Option<egui::TextureHandle>,
    last_size: (u32, u32),
    last_rgba: Option<Vec<u8>>,
    texture_dirty: bool,
    prev_threshold: u8,
    prev_invert: bool,

    // Status
    error_msg: Option<String>,
    status_msg: Option<String>,

    // Window positioning
    window_id: Option<String>,
    resize_to: Option<(f32, f32)>,
    reposition_to: Option<(i32, i32)>,

    // Channels
    action_rx: mpsc::Receiver<HotkeyAction>,
    region_rx: mpsc::Receiver<Result<SelectionResult, String>>,
    region_tx: mpsc::Sender<Result<SelectionResult, String>>,

}

impl ThresholdApp {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        action_rx: mpsc::Receiver<HotkeyAction>,
        region_rx: mpsc::Receiver<Result<SelectionResult, String>>,
        region_tx: mpsc::Sender<Result<SelectionResult, String>>,
        region_file: PathBuf,
        display: DisplayServer,
        default_threshold: u8,
        invert: bool,
        always_on_top: bool,
        panel_width: f32,
    ) -> Self {
        let region = capture::load_region(&region_file).ok();
        let window_id = Self::get_own_window_id(cc);
        if let Some(id) = &window_id {
            eprintln!("[threshold-filter] Our window ID: {id}");
        }

        let status = if region.is_some() {
            "Region loaded. Press F10 to select window + region.".to_string()
        } else {
            "Press F10 to select window + region.".to_string()
        };

        Self {
            threshold: default_threshold,
            invert,
            always_on_top,
            panel_collapsed: false,
            panel_width,
            display,
            target_window_id: None,
            target_crop: None,
            region,
            region_file,
            target_fps: 15.0,
            last_capture_time: Instant::now(),
            texture: None,
            last_size: (0, 0),
            last_rgba: None,
            texture_dirty: false,
            prev_threshold: default_threshold,
            prev_invert: invert,
            error_msg: None,
            status_msg: Some(status),
            window_id,
            resize_to: None,
            reposition_to: None,
            action_rx,
            region_rx,
            region_tx,
        }
    }

    fn get_own_window_id(cc: &eframe::CreationContext<'_>) -> Option<String> {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let handle = cc.window_handle().ok()?;
        match handle.as_raw() {
            RawWindowHandle::Xcb(h) => Some(h.window.get().to_string()),
            RawWindowHandle::Xlib(h) => Some(h.window.to_string()),
            _ => None,
        }
    }

    fn do_capture(&mut self) {
        if let Some(wid) = self.target_window_id {
            // X11: capture specific window via xcap (no self-capture)
            match capture::capture_window(wid, self.target_crop.as_ref()) {
                Ok((rgba, w, h)) => {
                    self.last_rgba = Some(rgba);
                    self.last_size = (w, h);
                    self.texture_dirty = true;
                    self.error_msg = None;
                }
                Err(e) => {
                    self.error_msg = Some(format!("Capture failed: {e}"));
                    self.target_window_id = None;
                }
            }
        } else if let Some(region) = &self.region {
            // Wayland fallback: screen capture via grim
            match capture::capture_region(region) {
                Ok((rgba, w, h)) => {
                    self.last_rgba = Some(rgba);
                    self.last_size = (w, h);
                    self.texture_dirty = true;
                    self.error_msg = None;
                    self.status_msg = Some(format!("Captured {w}x{h}"));
                }
                Err(e) => {
                    self.error_msg = Some(format!("Capture failed: {e}"));
                }
            }
        } else {
            self.error_msg = Some("No region selected. Press F10 first.".into());
        }
    }

    fn start_region_select(&self) {
        let region_file = self.region_file.clone();
        let tx = self.region_tx.clone();
        let display = self.display;
        std::thread::spawn(move || {
            match capture::select_region(display) {
                Ok(result) => {
                    if let Err(e) = capture::save_region(&region_file, &result.screen_region) {
                        eprintln!("[threshold-filter] Save region failed: {e}");
                    }
                    let _ = tx.send(Ok(result));
                }
                Err(e) => {
                    eprintln!("[threshold-filter] Region select failed: {e}");
                    let _ = tx.send(Err(format!("{e}")));
                }
            }
        });
    }

    fn poll_channels(&mut self) {
        // Poll hotkey actions
        while let Ok(action) = self.action_rx.try_recv() {
            match action {
                HotkeyAction::ToggleOnTop => {
                    self.always_on_top = !self.always_on_top;
                    self.status_msg = Some(if self.always_on_top {
                        "Always on top: ON".to_string()
                    } else {
                        "Always on top: OFF".to_string()
                    });
                }
            }
        }

        // Poll selection results
        while let Ok(result) = self.region_rx.try_recv() {
            match result {
                Ok(sel) => {
                    let r = &sel.screen_region;
                    self.status_msg = Some(format!("Capturing {}x{} at ({},{})", r.w, r.h, r.x, r.y));
                    self.error_msg = None;

                    // Store window capture info (X11)
                    self.target_window_id = sel.window_id;
                    self.target_crop = sel.crop;

                    // Show panel, but shift window left so image area aligns 1:1 with region
                    self.panel_collapsed = false;
                    self.resize_to = Some((r.w as f32, r.h as f32));
                    self.reposition_to = Some((r.x - self.panel_width as i32, r.y));

                    self.region = Some(sel.screen_region);
                }
                Err(msg) => {
                    self.error_msg = Some(format!("Selection failed: {msg}"));
                }
            }
        }
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        if self.threshold != self.prev_threshold || self.invert != self.prev_invert {
            self.texture_dirty = true;
            self.prev_threshold = self.threshold;
            self.prev_invert = self.invert;
        }

        if !self.texture_dirty {
            return;
        }

        if let Some(rgba) = &self.last_rgba {
            let (w, h) = self.last_size;
            if w == 0 || h == 0 {
                return;
            }

            let mut processed = rgba.clone();
            processing::apply_threshold(&mut processed, self.threshold);

            if self.invert {
                for pixel in processed.chunks_exact_mut(4) {
                    pixel[0] = 255 - pixel[0];
                    pixel[1] = 255 - pixel[1];
                    pixel[2] = 255 - pixel[2];
                }
            }

            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize],
                &processed,
            );
            match &mut self.texture {
                Some(tex) => tex.set(color_image, egui::TextureOptions::NEAREST),
                None => {
                    self.texture = Some(ctx.load_texture(
                        "capture",
                        color_image,
                        egui::TextureOptions::NEAREST,
                    ));
                }
            }
            self.texture_dirty = false;
        }
    }
}

impl eframe::App for ThresholdApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let frame_duration = Duration::from_secs_f32(1.0 / self.target_fps);
        ctx.request_repaint_after(frame_duration);

        // Always-on-top
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            if self.always_on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            },
        ));

        self.poll_channels();

        // Window positioning
        if let Some((w, h)) = self.resize_to.take() {
            let panel_w = if self.panel_collapsed { 0.0 } else { self.panel_width };
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                egui::vec2(w + panel_w, h),
            ));
        }
        if let Some((x, y)) = self.reposition_to.take() {
            if let Some(id) = &self.window_id {
                let _ = Command::new("xdotool")
                    .args(["windowmove", id, &x.to_string(), &y.max(0).to_string()])
                    .status();
            }
        }

        // Auto-capture at target FPS when we have a target window
        if self.target_window_id.is_some()
            && self.last_capture_time.elapsed() >= frame_duration
        {
            self.do_capture();
            self.last_capture_time = Instant::now();
        }

        self.update_texture(ctx);

        // Left control panel (collapsible)
        if !self.panel_collapsed {
            egui::SidePanel::left("controls")
                .exact_width(self.panel_width)
                .resizable(false)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // Collapse button
                        if ui.button("\u{2261}").on_hover_text("Collapse panel").clicked() {
                            self.panel_collapsed = true;
                        }
                        ui.separator();

                        // Vertical threshold slider
                        ui.label("Thr");
                        let mut t = self.threshold as f32;
                        if ui
                            .add_sized(
                                [self.panel_width - 10.0, 120.0],
                                egui::Slider::new(&mut t, 0.0..=255.0)
                                    .vertical()
                                    .integer()
                                    .show_value(true),
                            )
                            .changed()
                        {
                            self.threshold = t as u8;
                        }

                        ui.separator();

                        // Buttons
                        if ui.button("Sel").on_hover_text("Select window + region (F10)").clicked() {
                            self.start_region_select();
                        }

                        let mut inv = self.invert;
                        if ui.checkbox(&mut inv, "Inv").on_hover_text("Invert B/W").changed() {
                            self.invert = inv;
                        }

                        let mut aot = self.always_on_top;
                        if ui.checkbox(&mut aot, "Top").on_hover_text("Always on top (F8)").changed() {
                            self.always_on_top = aot;
                        }

                        ui.separator();
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });
        }

        // Central panel: image display
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
            // Only show expand button when NOT live-capturing (otherwise it eats image pixels)
            if self.panel_collapsed && self.texture.is_none()
                && ui.button("\u{2261}").on_hover_text("Expand panel").clicked()
            {
                self.panel_collapsed = false;
            }

            if let Some(tex) = &self.texture {
                let available = ui.available_size();
                // Fill available space exactly — window is sized to match the region 1:1
                ui.image(egui::load::SizedTexture::new(tex.id(), available));
            } else if let Some(err) = &self.error_msg {
                ui.centered_and_justified(|ui| {
                    ui.label(format!("{err}\n\nPress F10 to select window + region."));
                });
            } else if let Some(status) = &self.status_msg {
                ui.centered_and_justified(|ui| {
                    ui.label(status.as_str());
                });
            }
        });
    }
}
