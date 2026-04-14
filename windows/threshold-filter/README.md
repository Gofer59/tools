# threshold-filter

Live screen threshold filter for OCR preprocessing on Windows. Select any application window, draw a rectangle over the text area, and get a real-time black-and-white thresholded view. Dark backgrounds become black, bright text becomes white -- ideal for improving OCR accuracy on low-contrast or noisy text.

No external tools needed: window selection and region drawing are built into the app.

## Platform

Windows 10/11

## Prerequisites

- Rust toolchain ([https://rustup.rs](https://rustup.rs))

## Installation

```powershell
cargo build --release
```

The executable is at `target\release\threshold-filter.exe`. Copy it wherever you like:

```powershell
mkdir %USERPROFILE%\.local\bin 2>nul
copy target\release\threshold-filter.exe %USERPROFILE%\.local\bin\
```

## Usage

Launch `threshold-filter.exe` by double-clicking it or from a terminal:

```powershell
threshold-filter.exe
```

### Workflow

1. On first launch, a list of open windows appears -- click the one you want to capture
2. The window's full content appears as a preview -- **drag a rectangle** over the text area you want to threshold
3. The app resizes to match the selected region and positions itself over it
4. A live thresholded view updates at ~15 FPS
5. Adjust the **Thr** slider to fine-tune the black/white cutoff (0--255)
6. Check **Inv** to invert black and white
7. Click **Sel** (or press **F10**) to pick a different window and area
8. Click **Quit** to close

### UI Layout

Controls are in a narrow vertical panel on the left side. The thresholded image fills the remaining space on the right. The panel is scrollable when the window is too short to show all controls.

```
+------+-----------------------------+
| Thr  |                             |
| |==| |    Thresholded image        |
| |  | |    (fills remaining space)  |
| |==| |                             |
|      |                             |
| Sel  |                             |
| Inv  |                             |
| Top  |                             |
| Move |                             |
| < >  |                             |
| /\ \/|                             |
| Quit |                             |
+------+-----------------------------+
```

### Controls

| Control | Description |
|---------|-------------|
| **Thr** (slider 0--255) | Brightness threshold. Pixels brighter than this value become white; darker ones become black. Default: 128 |
| **Sel** button | Pick a new window and area. Same as pressing F10 |
| **Inv** checkbox | Invert black and white |
| **Top** checkbox | Keep the window above all other windows (toggle via hotkey also minimizes/restores the overlay) |
| **Move** buttons (`< > /\ \/`) | Nudge the overlay window by 20 pixels per click |
| **Quit** button | Close the application |

### Global Hotkeys

| Key | Action |
|-----|--------|
| **F10** | Re-select window and area |
| **F9** | Toggle always-on-top + minimize/restore overlay |

Hotkeys work globally -- they are detected even when another window is focused (e.g. a game), including games with anti-cheat software such as Genshin Impact. Hotkeys use Win32 `RegisterHotKey` (not a low-level hook), so they cannot be blocked by anti-cheat. Configurable via the config file.

## Configuration

Settings are loaded from a TOML config file:

```
%APPDATA%\threshold-filter\config.toml
```

A default config file is created automatically on first launch. Edit it with any text editor, then restart the app to apply changes.

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

### Supported key names

`F1`--`F12`, `Escape`/`Esc`, `Tab`, `Enter`/`Return`, `Space`, `Backspace`, `Delete`, `Home`, `End`, `A`--`Z` (or `KeyA`--`KeyZ`), arrow keys (`UpArrow`, `DownArrow`, `LeftArrow`, `RightArrow`), modifiers (`MetaLeft`, `AltLeft`, `ControlLeft`, `ShiftLeft`, and their `Right` variants), and raw keycodes (`191` or `Unknown(191)`).

Modifier combos use `+` as separator: `ControlLeft+F5`, `AltLeft+KeyU`.

## Desktop Shortcut

1. Press **Win + R**, type `shell:desktop`, press Enter
2. Right-click the desktop, select **New > Shortcut**
3. For the location, enter the full path to `threshold-filter.exe` (e.g. `C:\Users\YourName\.local\bin\threshold-filter.exe`)
4. Name it `Threshold Filter`
5. (Optional) Right-click the shortcut, select **Properties > Change Icon** to pick a custom icon

## Architecture

```
src/
  main.rs          Entry point, global hotkeys (Win32 RegisterHotKey), eframe window setup
  config.rs        TOML config: hotkeys, display defaults, %APPDATA% path resolution
  capture.rs       Per-window capture via xcap (WGC backend on Windows)
  processing.rs    Binary threshold (BT.601 luminance, fixed-point arithmetic)
  ui.rs            egui app: in-app window picker, drag-to-draw region selector,
                   scrollable left panel, always-on-top toggle, window nudge
```

The tool uses `xcap` with Windows Graphics Capture (WGC) to capture only the selected window's pixels -- no desktop background, no other windows, no self-capture. The captured image is cropped to the user-drawn sub-region, then a per-pixel binary threshold is applied and displayed via egui at ~15 FPS.

DPI scaling is handled automatically: the region drawer converts between physical (captured image) and logical (screen) coordinates using the ratio between the captured image size and the window's reported logical size.

### Crate dependencies

| Crate | Purpose |
|-------|---------|
| [eframe](https://crates.io/crates/eframe) / egui | GUI framework |
| [xcap](https://crates.io/crates/xcap) 0.8 (WGC) | Per-window screen capture, including DirectX/Vulkan games |
| [image](https://crates.io/crates/image) | Image cropping |
| [serde](https://crates.io/crates/serde) + [toml](https://crates.io/crates/toml) | Config file parsing |
| [rdev](https://crates.io/crates/rdev) | Global hotkey listener |
| [raw-window-handle](https://crates.io/crates/raw-window-handle) | Native window ID extraction (for self-capture filtering) |

## Known Limitations

- **Target window must stay open.** If the captured window is closed or minimized, capture fails gracefully and the display freezes on the last frame.
- **Mouse pointer may occasionally appear in captures.** `xcap` asks Windows Graphics Capture to exclude the cursor (`SetIsCursorCaptureEnabled(false)`), but on some Windows builds the flag is not honored reliably for per-window captures. If you see your pointer blink in the thresholded image, move it away from the target window or freeze capture by pausing the target app.
- **No multi-monitor DPI mixing.** DPI scaling is computed from the captured window; if the overlay and target are on monitors with different DPI, alignment may be slightly off.
- **Binary size is ~18 MB** due to the egui rendering backend.
- **Some windows may not appear in the picker** if they report zero size or have no title.
- **UWP/Store apps** may require running the tool as Administrator for WGC capture to work.

## License

MIT -- see [LICENSE](../../LICENSE)
