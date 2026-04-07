# Threshold Filter (SteamDeck)

Live screen threshold filter overlay for SteamDeck. Captures a screen region via grim/slurp, applies a BT.601 luminance threshold (converting to pure black/white), and displays the result in a borderless, always-on-top overlay window using egui. Designed for enhancing text readability in visual novels and games -- useful as an OCR preprocessing step or for reading low-contrast text. Auto-refreshes the captured region at a configurable frame rate. Supports both X11 (via xcap/slop) and Wayland (via grim/slurp).

## Platform

SteamDeck (SteamOS 3.x / KDE Plasma / Wayland)

Also works on standard Linux desktops (Arch, etc.) under X11 or Wayland.

## Prerequisites

- **Rust 1.82+** and **Cargo** (for building)
- System packages (installed automatically by `install.sh`):
  - `grim` (Wayland screenshot capture)
  - `slurp` (Wayland interactive region selection)
  - `maim`, `slop` (X11 fallback capture and selection)
  - `xdotool` (X11 window management)

> **SteamOS read-only filesystem:** The installer temporarily unlocks the filesystem with `steamos-readonly disable`, installs packages via pacman, then re-locks it. The filesystem is also re-locked automatically on installer failure.

## Installation

### Option A: Build on the SteamDeck (if cargo is available)

```bash
cd threshold-filter
chmod +x install.sh
./install.sh
```

The installer will build the binary if cargo is available and no pre-built binary exists.

### Option B: Cross-compile on your dev machine

Build on any x86_64 Linux PC with Rust 1.82+:

```bash
cd threshold-filter
cargo build --release
```

Copy the folder to the SteamDeck and run the installer:

```bash
scp -r threshold-filter/ deck@steamdeck:~/threshold-filter/
# On the SteamDeck:
cd ~/threshold-filter
chmod +x install.sh
./install.sh
```

The installer will:
1. Unlock the SteamOS filesystem and install `grim`, `slurp` via pacman
2. Re-lock the filesystem (also re-locks automatically on failure)
3. Check/offer to add your user to the `input` group
4. Install the binary to `~/.local/bin/threshold-filter-deck`
5. Create a KDE application menu entry

Ensure `~/.local/bin` is in your PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## Usage

```bash
threshold-filter-deck    # launch the overlay window
```

Or search "Threshold Filter" in the KDE application menu.

| Hotkey | Action |
|--------|--------|
| `F10` | **Select region** -- draw a rectangle with slurp (Wayland) or click a window + draw with slop (X11) |
| `F8` | **Toggle always-on-top** -- keep the overlay above other windows |

### UI controls

The overlay window has a collapsible left panel with:

- **Threshold slider** (0--255): drag to adjust the black/white cutoff
- **Sel** button: select a new screen region (same as F10)
- **Cap** button: capture the current region
- **Inv** toggle: invert the output (swap black and white)
- **Quit** button: close the application

The right panel displays the thresholded image, preserving the exact aspect ratio of the captured region. The image auto-refreshes at a configurable frame rate.

## Configuration

Config file: `~/.config/threshold-filter/config.toml`

Auto-created with defaults on first run. Edit with any text editor; restart for changes to take effect.

```toml
[hotkeys]
region_select   = "F10"       # Select new screen region
toggle_on_top   = "F8"        # Toggle always-on-top

[display]
default_threshold = 128       # 0-255 luminance cutoff
invert            = false     # swap black/white output
always_on_top     = true      # overlay stays above other windows
panel_width       = 50.0      # left control panel width in pixels
```

### Hotkey format

- Named keys: `F8`, `F9`, `F10`, `MetaLeft`, `KeyQ`, etc.
- Combos: `"MetaLeft+KeyQ"`, `"AltLeft+KeyU"`
- Raw keycodes from Steam Input: `"191"` or `"Unknown(191)"`

## Architecture

```
threshold-filter/
├── src/
│   ├── main.rs          3-thread orchestrator: rdev listener, hotkey dispatcher, egui main loop
│   ├── capture.rs       grim/slurp + slop/maim wrappers, Region struct, JSON persistence
│   ├── processing.rs    BT.601 luminance threshold (grayscale -> binary black/white)
│   ├── config.rs        TOML config loading with auto-create and defaults
│   └── ui.rs            egui: collapsible left panel, threshold slider, aspect-preserving image
├── Cargo.toml           Rust dependencies (eframe, egui, xcap, rdev, image, anyhow, toml, serde, ...)
└── install.sh           SteamDeck installer (pacman, input group, menu entry)
```

**Threading model:**
1. **rdev listener** (background thread) -- captures raw key events from `/dev/input`
2. **Hotkey dispatcher** (background thread) -- matches key combos, spawns slurp/slop, sends actions via channels
3. **egui main loop** (main thread) -- polls channels with `try_recv`, captures screen via grim/xcap, applies threshold, renders UI

**Display server auto-detection:**
- **Wayland:** `slurp` (region select) + `grim` (capture)
- **X11:** `xdotool` (window select) + `slop` (region select) + `xcap` (window capture)

**Installed files:**

| Path | Purpose |
|------|---------|
| `~/.local/bin/threshold-filter-deck` | Compiled binary |
| `~/.config/threshold-filter/config.toml` | Configuration (auto-created on first run) |
| `~/.local/share/threshold-filter/last_region.json` | Persisted capture region |
| `~/.local/share/applications/threshold-filter-deck.desktop` | KDE app menu entry |

## SteamOS Notes

- Packages installed via pacman (`grim`, `slurp`) don't survive SteamOS major updates -- re-run `install.sh` after updates
- User must be in the `input` group for rdev hotkeys: `sudo usermod -aG input $USER` (then reboot)
- Filesystem is read-only by default; `install.sh` handles unlock/re-lock via `steamos-readonly`
- Everything in `~/.local/` and `~/.config/` (binary, config, region file) survives updates
- The `input` group membership survives updates
- The installer re-locks the filesystem automatically, even if a step fails (via trap)

## Known Limitations

- Game Mode (Gamescope) may not forward key events to rdev -- use Desktop Mode
- The threshold filter is purely visual (black/white binary) -- it does not perform OCR itself
- On Wayland, screen capture uses `grim` which captures the entire output and crops; there may be a brief flicker on each capture
- The left panel collapses to save space but requires a minimum window height to display all controls
- No live preview during region selection (slurp/slop handle this independently)
- Requires Rust 1.82+ due to eframe/egui version requirements

## License

MIT -- see [LICENSE](../../LICENSE)
