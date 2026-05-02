use eframe::egui;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::capture::{self, WindowCrop};
use super::hotkey::HotkeyAction;
use super::processing;

// --- Platform-conditional imports ---

#[cfg(target_os = "linux")]
use std::process::Command;

// --- Config file path (mirrors what run_overlay() and save use) ---

fn config_json_path() -> Option<std::path::PathBuf> {
    dirs::data_local_dir().map(|d| d.join("threshold-filter").join("config.json"))
}

// --- JSON config struct for saving ---

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct SavedConfig {
    #[serde(default)]
    hotkey_reselect: String,
    #[serde(default)]
    hotkey_toggle: String,
    #[serde(default = "default_threshold")]
    default_threshold: u8,
    #[serde(default)]
    default_invert: bool,
    #[serde(default = "default_true")]
    default_aot: bool,
}

fn default_threshold() -> u8 { 128 }
fn default_true() -> bool { true }

// --- Selection result (used on Linux via channel, on Windows directly) ---

struct SelectionResult {
    window_id: u32,
    crop: WindowCrop,
    screen_x: i32,
    screen_y: i32,
}

// --- Windows-only: in-app selection state ---

#[cfg(target_os = "windows")]
enum WinSelection {
    Idle,
    PickingWindow {
        windows: Vec<(u32, String, i32, i32, u32, u32)>, // id, title, x, y, w, h
    },
    DrawingRegion {
        window_id: u32,
        win_x: i32,
        win_y: i32,
        preview: egui::TextureHandle,
        real_w: u32,
        real_h: u32,
        logical_w: u32,
        logical_h: u32,
        drag_start: Option<egui::Pos2>,
        drag_rect: Option<egui::Rect>,
    },
}

// --- Main app struct ---

pub struct ThresholdApp {
    threshold: u8,
    always_on_top: bool,
    invert: bool,
    action_rx: mpsc::Receiver<HotkeyAction>,
    key_reselect_name: String,
    key_toggle_top_name: String,
    texture: Option<egui::TextureHandle>,
    target_fps: f32,
    last_capture_time: Instant,
    last_rgba: Option<Vec<u8>>,
    last_capture_size: (u32, u32),
    texture_dirty: bool,
    prev_threshold: u8,
    prev_invert: bool,
    target_window_id: Option<u32>,
    target_crop: Option<WindowCrop>,
    error_msg: Option<String>,
    resize_to: Option<(f32, f32)>,
    reposition_to: Option<(i32, i32)>,
    window_id: Option<String>,
    pending_move: Option<(i32, i32)>,
    pending_on_top: Option<Instant>,
    align_pending: Option<(i32, i32)>,

    // Config panel fields
    cfg_hotkey_reselect: String,
    cfg_hotkey_toggle: String,
    cfg_default_thr: u8,
    cfg_default_invert: bool,
    cfg_default_aot: bool,
    cfg_status_msg: String,
    cfg_status_time: Option<Instant>,

    // Linux-only: background thread selection
    #[cfg(target_os = "linux")]
    selection_rx: Option<mpsc::Receiver<anyhow::Result<SelectionResult>>>,

    // Windows-only: in-app selection UI
    #[cfg(target_os = "windows")]
    win_selection: WinSelection,
}

const PANEL_WIDTH: f32 = 110.0;
const MOVE_STEP: i32 = 20;

impl ThresholdApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        action_rx: mpsc::Receiver<HotkeyAction>,
        key_reselect_name: String,
        key_toggle_top_name: String,
        default_threshold: u8,
        invert: bool,
        always_on_top: bool,
    ) -> Self {
        let window_id = Self::get_own_window_id(cc);
        if let Some(id) = &window_id {
            log::info!("Our window ID: {id}");
        }

        // Load saved config for cfg_* fields
        let saved = config_json_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<SavedConfig>(&s).ok())
            .unwrap_or_default();

        Self {
            threshold: default_threshold,
            always_on_top,
            invert,
            action_rx,
            key_reselect_name: key_reselect_name.clone(),
            key_toggle_top_name: key_toggle_top_name.clone(),
            texture: None,
            target_fps: 15.0,
            last_capture_time: Instant::now(),
            last_rgba: None,
            last_capture_size: (0, 0),
            texture_dirty: false,
            prev_threshold: default_threshold,
            prev_invert: invert,
            target_window_id: None,
            target_crop: None,
            error_msg: None,
            resize_to: None,
            reposition_to: None,
            window_id,
            pending_move: None,
            pending_on_top: None,
            align_pending: None,
            cfg_hotkey_reselect: if saved.hotkey_reselect.is_empty() {
                key_reselect_name
            } else {
                saved.hotkey_reselect
            },
            cfg_hotkey_toggle: if saved.hotkey_toggle.is_empty() {
                key_toggle_top_name
            } else {
                saved.hotkey_toggle
            },
            cfg_default_thr: saved.default_threshold,
            cfg_default_invert: saved.default_invert,
            cfg_default_aot: saved.default_aot,
            cfg_status_msg: String::new(),
            cfg_status_time: None,
            #[cfg(target_os = "linux")]
            selection_rx: None,
            #[cfg(target_os = "windows")]
            win_selection: WinSelection::Idle,
        }
    }

    fn get_own_window_id(cc: &eframe::CreationContext<'_>) -> Option<String> {
        use raw_window_handle::HasWindowHandle;
        use raw_window_handle::RawWindowHandle;
        let handle = cc.window_handle().ok()?;
        match handle.as_raw() {
            #[cfg(target_os = "linux")]
            RawWindowHandle::Xcb(h) => Some(h.window.get().to_string()),
            #[cfg(target_os = "linux")]
            RawWindowHandle::Xlib(h) => Some(h.window.to_string()),
            #[cfg(target_os = "windows")]
            RawWindowHandle::Win32(h) => Some(format!("{}", h.hwnd.get() as isize)),
            _ => None,
        }
    }

    fn move_window(&mut self, dx: i32, dy: i32) {
        #[cfg(target_os = "linux")]
        {
            if let Some(id) = &self.window_id {
                match Command::new("xdotool")
                    .args(["windowmove", "--relative", id, &dx.to_string(), &dy.to_string()])
                    .status()
                {
                    Ok(s) if !s.success() => log::warn!("xdotool windowmove exited: {s}"),
                    Err(e) => log::warn!("xdotool windowmove failed: {e}"),
                    _ => {}
                }
            }
        }
        #[cfg(target_os = "windows")]
        {
            self.pending_move = Some((dx, dy));
        }
    }

    fn is_selecting(&self) -> bool {
        #[cfg(target_os = "linux")]
        { self.selection_rx.is_some() }
        #[cfg(target_os = "windows")]
        { !matches!(self.win_selection, WinSelection::Idle) }
    }

    fn start_selection(&mut self) {
        if self.is_selecting() {
            return;
        }
        self.error_msg = None;

        #[cfg(target_os = "linux")]
        {
            let (tx, rx) = mpsc::channel();
            self.selection_rx = Some(rx);
            std::thread::spawn(move || {
                let result = do_selection_linux();
                let _ = tx.send(result);
            });
        }

        #[cfg(target_os = "windows")]
        {
            let own_id: u32 = self.window_id.as_ref()
                .and_then(|s| s.parse::<isize>().ok())
                .map(|v| v as u32)
                .unwrap_or(u32::MAX);

            let mut windows = Vec::new();
            if let Ok(all) = xcap::Window::all() {
                for w in all {
                    let id = w.id().unwrap_or(0);
                    let title = w.title().unwrap_or_default();
                    let ww = w.width().unwrap_or(0);
                    let wh = w.height().unwrap_or(0);
                    let wx = w.x().unwrap_or(0);
                    let wy = w.y().unwrap_or(0);
                    if ww > 0 && wh > 0 && !title.is_empty()
                        && id != own_id
                        && !w.is_minimized().unwrap_or(true)
                    {
                        windows.push((id, title, wx, wy, ww, wh));
                    }
                }
            }
            self.win_selection = WinSelection::PickingWindow { windows };
        }
    }

    fn apply_selection(&mut self, result: SelectionResult) {
        log::info!(
            "Capturing window {} crop {}x{} at ({},{})",
            result.window_id,
            result.crop.width, result.crop.height,
            result.crop.x, result.crop.y
        );

        let crop_w = result.crop.width;
        let crop_h = result.crop.height;
        let screen_x = result.screen_x;
        let screen_y = result.screen_y;

        self.target_crop = Some(result.crop);
        self.target_window_id = Some(result.window_id);
        self.error_msg = None;

        // Pixel-perfect alignment: size the window and position it so the
        // content area (right of the sidebar) sits exactly at (screen_x, screen_y).
        self.resize_to = Some((crop_w as f32, crop_h as f32));
        self.reposition_to = Some((screen_x, screen_y));
        self.align_pending = Some((screen_x, screen_y));
    }

    #[cfg(target_os = "linux")]
    fn poll_selection(&mut self) {
        if let Some(rx) = &self.selection_rx {
            match rx.try_recv() {
                Ok(Ok(result)) => {
                    self.apply_selection(result);
                    self.selection_rx = None;
                }
                Ok(Err(e)) => {
                    let msg = format!("Selection failed: {e}");
                    log::warn!("{msg}");
                    self.error_msg = Some(msg);
                    self.selection_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.error_msg = Some("Selection ended unexpectedly".to_string());
                    self.selection_rx = None;
                }
            }
        }
    }

    fn do_capture(&mut self) {
        let (Some(wid), Some(crop)) = (self.target_window_id, self.target_crop.as_ref()) else {
            return;
        };
        match capture::capture_window(wid, Some(crop)) {
            Ok((rgba, w, h)) => {
                self.last_rgba = Some(rgba);
                self.last_capture_size = (w, h);
                self.texture_dirty = true;
            }
            Err(e) => {
                log::warn!("Capture failed: {e:#}");
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
            let (w, h) = self.last_capture_size;
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
                        "capture", color_image, egui::TextureOptions::NEAREST,
                    ));
                }
            }
            self.texture_dirty = false;
        }
    }

    fn save_config(&mut self) {
        let cfg = SavedConfig {
            hotkey_reselect: self.cfg_hotkey_reselect.clone(),
            hotkey_toggle: self.cfg_hotkey_toggle.clone(),
            default_threshold: self.cfg_default_thr,
            default_invert: self.cfg_default_invert,
            default_aot: self.cfg_default_aot,
        };
        match config_json_path() {
            None => {
                self.cfg_status_msg = "Error: cannot determine config path".to_string();
            }
            Some(path) => {
                let result = (|| -> anyhow::Result<()> {
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    let json = serde_json::to_string_pretty(&cfg)?;
                    std::fs::write(&path, json)?;
                    Ok(())
                })();
                match result {
                    Ok(()) => {
                        self.cfg_status_msg = "saved".to_string();
                    }
                    Err(e) => {
                        self.cfg_status_msg = format!("Error: {e}");
                    }
                }
            }
        }
        self.cfg_status_time = Some(Instant::now());
    }
}

impl eframe::App for ThresholdApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let frame_duration = Duration::from_secs_f32(1.0 / self.target_fps);
        ctx.request_repaint_after(frame_duration);

        // Poll global hotkeys from listener
        while let Ok(action) = self.action_rx.try_recv() {
            match action {
                HotkeyAction::Reselect => {
                    if !self.is_selecting() {
                        self.start_selection();
                    }
                }
                HotkeyAction::ToggleOnTop => {
                    self.always_on_top = !self.always_on_top;
                    if self.always_on_top {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        self.pending_on_top = Some(
                            Instant::now() + Duration::from_millis(150),
                        );
                    } else {
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                            egui::WindowLevel::Normal,
                        ));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                }
            }
        }

        // Delayed AlwaysOnTop after restore from minimized
        if let Some(deadline) = self.pending_on_top {
            if Instant::now() >= deadline {
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                    egui::WindowLevel::AlwaysOnTop,
                ));
                self.pending_on_top = None;
            }
        }

        // Linux: poll background selection thread
        #[cfg(target_os = "linux")]
        self.poll_selection();

        // Resize + reposition after selection
        if let Some((w, h)) = self.resize_to.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                PANEL_WIDTH + w,
                h,
            )));
        }
        if let Some((x, y)) = self.reposition_to.take() {
            let target_content_x = x;
            let target_content_y = y;

            #[cfg(target_os = "linux")]
            {
                if let Some(id) = &self.window_id {
                    let adjusted_x = target_content_x - PANEL_WIDTH as i32;
                    match Command::new("xdotool")
                        .args([
                            "windowmove",
                            id,
                            &adjusted_x.to_string(),
                            &target_content_y.max(0).to_string(),
                        ])
                        .status()
                    {
                        Ok(s) if !s.success() => log::warn!("xdotool windowmove exited: {s}"),
                        Err(e) => log::warn!("xdotool windowmove failed: {e}"),
                        _ => {}
                    }
                }
            }
            #[cfg(target_os = "windows")]
            {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                    (target_content_x - PANEL_WIDTH as i32) as f32,
                    target_content_y.max(0) as f32,
                )));
            }
        }

        // Pixel-perfect one-frame delta correction
        if let Some((target_cx, target_cy)) = self.align_pending.take() {
            let inner = ctx.input(|i| i.viewport().inner_rect);
            let outer = ctx.input(|i| i.viewport().outer_rect);
            if let (Some(inner), Some(outer)) = (inner, outer) {
                let actual_content_x = inner.min.x + PANEL_WIDTH;
                let actual_content_y = inner.min.y;
                let dx = target_cx as f32 - actual_content_x;
                let dy = target_cy as f32 - actual_content_y;
                if dx.abs() > 0.5 || dy.abs() > 0.5 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                        outer.min.x + dx,
                        outer.min.y + dy,
                    )));
                }
            }
        }

        // Process pending move (Windows: ViewportCommand; Linux: already done in move_window)
        if let Some((dx, dy)) = self.pending_move.take() {
            if let Some(rect) = ctx.input(|i| i.viewport().outer_rect) {
                let new_x = rect.min.x + dx as f32;
                let new_y = rect.min.y + dy as f32;
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                    egui::pos2(new_x, new_y.max(0.0)),
                ));
            }
        }

        // Auto-select on first frame
        if self.target_window_id.is_none() && !self.is_selecting() && self.error_msg.is_none() {
            self.start_selection();
        }

        // Capture at target FPS
        if self.target_window_id.is_some()
            && !self.is_selecting()
            && self.last_capture_time.elapsed() >= frame_duration
        {
            self.do_capture();
            self.last_capture_time = Instant::now();
        }

        self.update_texture(ctx);

        // Clear status message after 2 seconds
        if let Some(t) = self.cfg_status_time {
            if t.elapsed() >= Duration::from_secs(2) {
                self.cfg_status_msg.clear();
                self.cfg_status_time = None;
            }
        }

        // Left control panel — persistent, always visible, no CollapsingHeader
        egui::SidePanel::left("controls")
            .min_width(PANEL_WIDTH)
            .resizable(false)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // 1. Threshold vertical slider
                    let mut t = self.threshold as f32;
                    if ui
                        .add_sized(
                            [45.0, 120.0],
                            egui::Slider::new(&mut t, 0.0_f32..=255.0)
                                .vertical()
                                .integer()
                                .show_value(true),
                        )
                        .changed()
                    {
                        self.threshold = t as u8;
                    }

                    // 2. Sel button
                    let btn = ui.add_sized(
                        [ui.available_width(), 0.0],
                        egui::Button::new("Sel").min_size(egui::vec2(ui.available_width(), 0.0)),
                    );
                    if btn
                        .on_hover_text(format!(
                            "Select window + area ({})",
                            self.key_reselect_name
                        ))
                        .clicked()
                        && !self.is_selecting()
                    {
                        self.start_selection();
                    }

                    // 3. Inv checkbox
                    ui.checkbox(&mut self.invert, "Inv")
                        .on_hover_text("Invert B/W");

                    // 4. Top checkbox
                    let prev_aot = self.always_on_top;
                    ui.checkbox(&mut self.always_on_top, "Top")
                        .on_hover_text(format!(
                            "Always on top ({})",
                            self.key_toggle_top_name
                        ));
                    if self.always_on_top != prev_aot {
                        let level = if self.always_on_top {
                            egui::WindowLevel::AlwaysOnTop
                        } else {
                            egui::WindowLevel::Normal
                        };
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
                    }

                    // 5-8. Move buttons
                    ui.horizontal(|ui| {
                        if ui.button("<").clicked() {
                            self.move_window(-MOVE_STEP, 0);
                        }
                        if ui.button(">").clicked() {
                            self.move_window(MOVE_STEP, 0);
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("/\\").clicked() {
                            self.move_window(0, -MOVE_STEP);
                        }
                        if ui.button("\\/").clicked() {
                            self.move_window(0, MOVE_STEP);
                        }
                    });

                    // 9. Quit button
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }

                    ui.separator();

                    // 10. Hotkeys label
                    ui.add(egui::Label::new(
                        egui::RichText::new("Hotkeys:").small().weak(),
                    ));

                    // 11. Reselect hotkey text field
                    ui.add(
                        egui::TextEdit::singleline(&mut self.cfg_hotkey_reselect)
                            .desired_width(90.0),
                    );

                    // 12. Toggle hotkey text field
                    ui.add(
                        egui::TextEdit::singleline(&mut self.cfg_hotkey_toggle)
                            .desired_width(90.0),
                    );

                    // 13. Threshold label
                    ui.label("Threshold:");

                    // 14. Default threshold slider (horizontal)
                    ui.add(
                        egui::Slider::new(&mut self.cfg_default_thr, 0u8..=255)
                            .integer(),
                    );

                    // 15. Default invert checkbox
                    ui.checkbox(&mut self.cfg_default_invert, "Inv*")
                        .on_hover_text("Default invert on startup");

                    // 16. Default always-on-top checkbox
                    ui.checkbox(&mut self.cfg_default_aot, "Top*")
                        .on_hover_text("Default always-on-top");

                    // Save button + status
                    if ui
                        .add_sized(
                            [ui.available_width(), 0.0],
                            egui::Button::new("Save"),
                        )
                        .clicked()
                    {
                        self.save_config();
                    }
                    if !self.cfg_status_msg.is_empty() {
                        ui.label(&self.cfg_status_msg);
                    }
                });
            });

        // Image display / selection UI
        egui::CentralPanel::default().show(ctx, |ui| {
            #[cfg(target_os = "windows")]
            self.render_windows_selection(ctx, ui);

            #[cfg(not(target_os = "windows"))]
            self.render_capture_display(ui);
        });
    }
}

// --- Shared rendering ---

impl ThresholdApp {
    fn render_capture_display(&self, ui: &mut egui::Ui) {
        if let Some(tex) = &self.texture {
            let available = ui.available_size();
            ui.image(egui::load::SizedTexture::new(tex.id(), available));
        } else if self.is_selecting() {
            ui.centered_and_justified(|ui| {
                #[cfg(target_os = "linux")]
                ui.label(
                    "Step 1: Click on the window to capture\nStep 2: Draw a rectangle on the area you want",
                );
                #[cfg(target_os = "windows")]
                ui.label("Select a window from the list...");
            });
        } else if let Some(err) = &self.error_msg {
            ui.centered_and_justified(|ui| {
                ui.label(format!("{err}\n\nPress {} to retry.", self.key_reselect_name));
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(format!(
                    "Press {} to select a window and area.",
                    self.key_reselect_name
                ));
            });
        }
    }
}

// --- Windows-only: in-app selection UI ---

#[cfg(target_os = "windows")]
impl ThresholdApp {
    fn render_windows_selection(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        match &self.win_selection {
            WinSelection::Idle => {
                self.render_capture_display(ui);
            }
            WinSelection::PickingWindow { .. } => {
                self.render_window_picker(ctx, ui);
            }
            WinSelection::DrawingRegion { .. } => {
                self.render_region_drawer(ctx, ui);
            }
        }
    }

    fn render_window_picker(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Select a window to capture:");
        ui.separator();

        let WinSelection::PickingWindow { windows } = &self.win_selection else {
            return;
        };
        let windows = windows.clone();

        let mut picked = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (id, title, _x, _y, w, h) in &windows {
                let label = format!("{title}  ({w}x{h})");
                if ui.button(&label).clicked() {
                    picked = Some(*id);
                }
            }
        });

        if let Some(wid) = picked {
            if let Some((_id, _title, wx, wy, ww, wh)) =
                windows.iter().find(|(id, ..)| *id == wid)
            {
                if let Ok(all) = xcap::Window::all() {
                    if let Some(win) = all.into_iter().find(|w| w.id().unwrap_or(0) == wid) {
                        if let Ok(img) = win.capture_image() {
                            let (rw, rh) = img.dimensions();
                            log::info!(
                                "Window {wid}: logical {}x{}, captured {}x{}",
                                ww, wh, rw, rh
                            );
                            let color_img = egui::ColorImage::from_rgba_unmultiplied(
                                [rw as usize, rh as usize],
                                img.as_raw(),
                            );
                            let tex = ctx.load_texture(
                                "preview",
                                color_img,
                                egui::TextureOptions::NEAREST,
                            );
                            self.win_selection = WinSelection::DrawingRegion {
                                window_id: wid,
                                win_x: *wx,
                                win_y: *wy,
                                preview: tex,
                                real_w: rw,
                                real_h: rh,
                                logical_w: *ww,
                                logical_h: *wh,
                                drag_start: None,
                                drag_rect: None,
                            };
                            return;
                        }
                    }
                }
                self.error_msg = Some("Failed to capture window preview".to_string());
                self.win_selection = WinSelection::Idle;
            }
        }
    }

    fn render_region_drawer(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.label("Drag a rectangle on the area you want to threshold:");

        let (window_id, win_x, win_y, tex_id, real_w, real_h, logical_w, _logical_h) = {
            let WinSelection::DrawingRegion {
                window_id,
                win_x,
                win_y,
                preview,
                real_w,
                real_h,
                logical_w,
                logical_h,
                ..
            } = &self.win_selection
            else {
                return;
            };
            (
                *window_id,
                *win_x,
                *win_y,
                preview.id(),
                *real_w,
                *real_h,
                *logical_w,
                *logical_h,
            )
        };

        let available = ui.available_size();
        let scale_x = available.x / real_w as f32;
        let scale_y = available.y / real_h as f32;
        let scale = scale_x.min(scale_y).min(1.0);
        let display_w = real_w as f32 * scale;
        let display_h = real_h as f32 * scale;

        let (response, painter) =
            ui.allocate_painter(egui::vec2(display_w, display_h), egui::Sense::drag());
        let origin = response.rect.min;

        painter.image(
            tex_id,
            response.rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );

        if response.drag_started() {
            if let WinSelection::DrawingRegion { drag_start, .. } = &mut self.win_selection {
                *drag_start = response.interact_pointer_pos();
            }
        }

        let drag_start_pos = if let WinSelection::DrawingRegion { drag_start, .. } =
            &self.win_selection
        {
            *drag_start
        } else {
            None
        };

        if let Some(start) = drag_start_pos {
            if let Some(current) = response.interact_pointer_pos() {
                let rect = egui::Rect::from_two_pos(start, current);
                let clamped = rect.intersect(response.rect);
                painter.rect_stroke(
                    clamped,
                    0.0,
                    egui::Stroke::new(2.0, egui::Color32::RED),
                    egui::StrokeKind::Outside,
                );
                if let WinSelection::DrawingRegion { drag_rect, .. } = &mut self.win_selection {
                    *drag_rect = Some(clamped);
                }
            }
        }

        if response.drag_stopped() {
            let drag_rect =
                if let WinSelection::DrawingRegion { drag_rect, .. } = &self.win_selection {
                    *drag_rect
                } else {
                    None
                };

            if let Some(rect) = drag_rect {
                if rect.width() > 5.0 && rect.height() > 5.0 {
                    let crop_x = ((rect.min.x - origin.x) / scale) as u32;
                    let crop_y = ((rect.min.y - origin.y) / scale) as u32;
                    let crop_w = (rect.width() / scale) as u32;
                    let crop_h = (rect.height() / scale) as u32;

                    let dpi_scale = real_w as f32 / (logical_w as f32).max(1.0);
                    let screen_x = win_x + (crop_x as f32 / dpi_scale) as i32;
                    let screen_y = win_y + (crop_y as f32 / dpi_scale) as i32;

                    log::info!(
                        "Region: crop ({crop_x},{crop_y}) {crop_w}x{crop_h} physical, \
                         screen ({screen_x},{screen_y}) logical, dpi_scale={dpi_scale:.2}"
                    );

                    self.win_selection = WinSelection::Idle;
                    self.apply_selection(SelectionResult {
                        window_id,
                        crop: WindowCrop {
                            x: crop_x,
                            y: crop_y,
                            width: crop_w.max(1),
                            height: crop_h.max(1),
                        },
                        screen_x,
                        screen_y,
                    });
                    // Override resize with logical-pixel dimensions for InnerSize.
                    self.resize_to = Some((
                        (crop_w as f32 / dpi_scale).max(1.0),
                        (crop_h as f32 / dpi_scale).max(1.0),
                    ));
                } else {
                    self.win_selection = WinSelection::Idle;
                    self.start_selection();
                }
            } else {
                self.win_selection = WinSelection::Idle;
                self.start_selection();
            }
        }
    }
}

// --- Linux-only: xdotool + slop selection ---

#[cfg(target_os = "linux")]
fn do_selection_linux() -> anyhow::Result<SelectionResult> {
    use std::process::Command;

    // Step 1: click on the target window
    let output = Command::new("xdotool")
        .arg("selectwindow")
        .output()
        .map_err(|e| anyhow::anyhow!("xdotool selectwindow failed: {e}"))?;
    if !output.status.success() {
        anyhow::bail!("Window selection cancelled");
    }
    let wid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let window_id: u32 = wid_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid window ID: {wid_str}"))?;

    // Get window position
    let geo_output = Command::new("xdotool")
        .args(["getwindowgeometry", "--shell", &wid_str])
        .output()?;
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

    // Step 2: draw a sub-region with slop
    let slop_output = Command::new("slop")
        .arg("--format=%w %h %x %y")
        .output()
        .map_err(|e| anyhow::anyhow!("slop failed: {e}"))?;
    if !slop_output.status.success() {
        anyhow::bail!("Region selection cancelled");
    }
    let text = String::from_utf8_lossy(&slop_output.stdout);
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() < 4 {
        anyhow::bail!("unexpected slop output: {text}");
    }
    let slop_w: u32 = parts[0].parse()?;
    let slop_h: u32 = parts[1].parse()?;
    let slop_x: i32 = parts[2].parse()?;
    let slop_y: i32 = parts[3].parse()?;

    let crop_x = (slop_x - win_x).max(0) as u32;
    let crop_y = (slop_y - win_y).max(0) as u32;

    Ok(SelectionResult {
        window_id,
        crop: WindowCrop {
            x: crop_x,
            y: crop_y,
            width: slop_w,
            height: slop_h,
        },
        screen_x: slop_x,
        screen_y: slop_y,
    })
}
