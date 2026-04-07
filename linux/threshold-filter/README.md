# threshold-filter

Live screen threshold filter for OCR preprocessing. Click on an app window, then draw a rectangle to select the text area. The tool captures only that window's content (not the desktop or other windows) and displays a real-time binary threshold. Dark backgrounds become black, bright text becomes white.

Works on both **Linux (X11)** and **Windows**.

## Build

Requires the Rust toolchain (`cargo`).

### Linux

```bash
# Install build dependencies (Ubuntu/Debian)
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libxkbcommon-dev libgl-dev pkg-config

# Install runtime dependencies
sudo apt-get install xdotool slop

# Build and install
./install.sh
```

### Windows

```powershell
cargo build --release
copy target\release\threshold-filter.exe %USERPROFILE%\.local\bin\
```

No additional system tools needed — window selection and region drawing are built into the app on Windows.

#### Desktop shortcut

1. Press **Win + R**, type `shell:desktop`, press Enter
2. Right-click → **New → Shortcut**
3. Location: `%USERPROFILE%\.local\bin\threshold-filter.exe`
4. Name: `Threshold Filter`
5. (Optional) Right-click the shortcut → **Properties → Change Icon** to pick a custom icon

## Usage

```
threshold-filter
```

### Linux (X11)

1. **Step 1:** Click on the window you want to capture (`xdotool selectwindow`)
2. **Step 2:** Draw a rectangle on the area you want to threshold (`slop`)
3. The display window resizes to match and positions itself over the selected area
4. A live thresholded view updates at ~15 FPS
5. Press **F10** to select a different window and area

### Windows

1. Launch `threshold-filter.exe` (double-click the desktop shortcut or run from a terminal)
2. Click **Sel** in the left panel (or press **F10**)
3. A list of open windows appears — click the one you want to capture
4. The window's full content appears as a preview — **drag a rectangle** over the text area you want to threshold
5. The app resizes to match the selected area and positions itself over it
6. A live thresholded view updates at ~15 FPS
7. Adjust the **Thr** slider to fine-tune the black/white cutoff
8. Click **Sel** again (or press **F10**) to pick a different window and area
9. Click **Quit** to close

### UI Layout

Controls are in a narrow vertical panel on the left side. The thresholded image fills the remaining space on the right, preserving exact aspect ratio. The panel is scrollable when the window is too short to show all controls.

```
+------+-----------------------------+
| Thr  |                             |
| |==| |    Thresholded image        |
| |  | |    (exact aspect ratio)     |
| |==| |                             |
|      |                             |
| Sel  |                             |
| Top  |                             |
| Move |                             |
| < >  |                             |
| /\ \/|                             |
| Quit |                             |
+------+-----------------------------+
 left         right: image fills
 panel        remaining space
(scrollable)
```

### Controls

- **Thr (vertical slider 0-255):** Adjust the brightness threshold. Default: 128.
- **Sel button:** Pick a new window and area (default: F10)
- **Inv checkbox:** Invert black/white
- **Top checkbox:** Keep the window above other windows (default: F9 to toggle)
- **Move buttons (< > /\ \/):** Nudge the display window 20px per click (Linux: via `xdotool`)
- **Quit button:** Close the application

### Hotkeys (default)

| Key | Action |
|-----|--------|
| F10 | Select window + area |
| F9  | Toggle always-on-top |

Hotkeys work globally — they are detected even when another window is focused (e.g. a game). Configurable via the config file (see below).

## Configuration

Settings are loaded from a TOML config file:
- **Linux:** `~/.config/threshold-filter/config.toml`
- **Windows:** `%APPDATA%\threshold-filter\config.toml`

A default config file is created on first launch. Edit it to customize hotkeys and display defaults, then restart the app.

### Default config

```toml
[hotkeys]
# Key names: F1-F12, Escape, Tab, Space, A-Z, etc.
# Modifier combos: MetaLeft+KeyQ, AltLeft+KeyU, ControlLeft+KeyR
# Raw keycodes: "191" or "Unknown(191)"
region_select   = "F10"
toggle_on_top   = "F9"

[display]
default_threshold = 128       # 0-255
invert            = false     # swap black/white
always_on_top     = true
```

### Key names

`F1`-`F12`, `Escape`/`Esc`, `Tab`, `Enter`/`Return`, `Space`, `Backspace`, `Delete`, `Home`, `End`, `A`-`Z` (or `KeyA`-`KeyZ`), arrow keys (`UpArrow`, `DownArrow`, `LeftArrow`, `RightArrow`), modifiers (`MetaLeft`, `AltLeft`, `ControlLeft`, etc.), and raw keycodes (`191` or `Unknown(191)`).

## How It Works

The tool uses `xcap::Window::capture_image()` to capture only the selected window's pixels — no desktop background, no other windows, no self-capture. The captured image is cropped to the user-drawn sub-region, then a per-pixel binary threshold (BT.601 luminance >= slider -> white, else -> black) is applied and displayed at ~15 FPS.

**Platform differences:**
- **Linux:** Selection uses external tools (`xdotool` + `slop`). Window movement uses `xdotool windowmove`.
- **Windows:** Selection is fully in-app (window list + drag-to-draw rectangle). Window movement uses egui `ViewportCommand`.

## Architecture

```
src/
  main.rs          Entry point, config loading, eframe window setup
  config.rs        TOML config file: hotkeys, display defaults, cross-platform paths
  capture.rs       Per-window capture via xcap::Window + sub-region crop
  processing.rs    Binary threshold filter (BT.601 luminance, fixed-point)
  ui.rs            egui app: platform-conditional selection, scrollable left panel
```

## Runtime Dependencies

### Linux
- `xdotool` — window selection, geometry query, and window movement
- `slop` — interactive sub-region rectangle drawing

### Windows
- None — all selection UI is built into the app

## Known Limitations

- **Linux:** Requires X11 (`xdotool` and `slop` do not work on Wayland). For SteamDeck/Wayland, use the `steamdeck/` variant. Global hotkeys require the user to be in the `input` group (`sudo usermod -aG input $USER`, then reboot).
- **Target window must stay open:** If the captured window is closed, capture fails gracefully
- **Binary size:** ~18 MB due to egui rendering backend

## Crate Dependencies

- [eframe](https://crates.io/crates/eframe) (egui) — GUI framework
- [xcap](https://crates.io/crates/xcap) — per-window screen capture (Linux + Windows)
- [image](https://crates.io/crates/image) — image cropping
- [serde](https://crates.io/crates/serde) + [toml](https://crates.io/crates/toml) — config file parsing
- [rdev](https://crates.io/crates/rdev) — global hotkey listener (works even when another app is focused)
- [raw-window-handle](https://crates.io/crates/raw-window-handle) — native window ID extraction
