# Screen OCR (SteamDeck)

Hotkey-triggered screen region OCR with text-to-speech for SteamDeck. Designed for visual novels: select the dialogue text box once with F10, then press F9 for each new line to OCR it, copy the extracted text to clipboard, and hear it read aloud via Piper TTS. Auto-detects X11 or Wayland and uses the appropriate capture tools (slurp/grim on Wayland, slop/maim on X11). Fully offline -- Tesseract for OCR, Piper for TTS, no cloud services or API keys.

## Platform

SteamDeck (SteamOS 3.x / KDE Plasma / Wayland)

Also works on standard Linux desktops (Debian, Ubuntu, Arch) under X11 or Wayland.

## Prerequisites

- **Rust 1.70+** and **Cargo** (for building)
- **Python 3** (pre-installed on SteamOS and most Linux distributions)
- System packages (installed automatically by `install.sh`):
  - **Wayland (SteamDeck):** `grim`, `slurp`, `wl-clipboard`
  - **X11 (fallback):** `maim`, `slop`, `xclip`, `xdotool`
  - `tesseract` + `tesseract-data-eng` (OCR engine)
  - `alsa-lib` (build dependency for rdev)
- **voice-speak** (optional, for TTS) -- install from the sibling `voice-speak/` directory

> **SteamOS read-only filesystem:** On SteamOS, system packages require temporarily unlocking the filesystem:
> ```bash
> sudo steamos-readonly disable
> sudo pacman -S --noconfirm tesseract tesseract-data-eng grim slurp wl-clipboard alsa-lib
> sudo steamos-readonly enable
> ```

## Installation

```bash
cd screen-ocr
chmod +x install.sh
./install.sh
```

The installer will:
1. Detect your display server (X11 or Wayland) and package manager (apt or pacman)
2. Install system packages for screen capture, clipboard, and OCR
3. Check for voice-speak TTS (optional -- OCR works without it)
4. Create a Python venv at `~/.local/share/screen-ocr/venv/` with `pytesseract` and `Pillow`
5. Build the Rust binary in release mode
6. Install everything to `~/.local/bin/`

Ensure `~/.local/bin` is in your PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### SteamOS: Install voice-speak first (recommended)

For TTS support, install the sibling voice-speak tool before screen-ocr:

```bash
cd ../voice-speak
chmod +x install.sh
./install.sh
```

## Usage

```bash
screen-ocr          # runs in foreground, Ctrl-C to quit
```

| Hotkey | Action |
|--------|--------|
| `F10` | **Select region** -- draw a rectangle with the cursor; region geometry is saved to disk |
| `F9` | **Quick capture** -- re-capture the saved region instantly, OCR, copy to clipboard, speak aloud |
| `F11` | **Stop TTS** -- stop speech playback |

### Visual novel workflow

1. Press **F10** to draw a rectangle around the dialogue text box (done once)
2. Press **F9** each time the dialogue advances -- the saved region is re-captured instantly
3. Text is extracted, copied to clipboard, and spoken aloud automatically
4. Press **F10** again if the text box moves or you switch games

### Cancel a selection

Press **Escape** while the crosshair is visible (during F10) to cancel.

### Autostart on login

Create `~/.config/autostart/screen-ocr.desktop`:

```ini
[Desktop Entry]
Type=Application
Name=screen-ocr
Exec=screen-ocr
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
```

## Configuration

All configuration is done by editing `src/main.rs` in the `Config` struct, then rebuilding with `./install.sh`.

### Hotkeys

```rust
quick_capture_key: Key::F9,     // re-capture stored region
select_region_key: Key::F10,    // interactive selection
stop_tts_key: Key::F11,         // stop TTS playback
```

### Delivery mode

```rust
delivery_mode: DeliveryMode::Clipboard,  // "Clipboard" | "Type" | "Both"
```

| Mode | Behavior |
|------|----------|
| `Clipboard` | Copies text to system clipboard (default) |
| `Type` | Types text at cursor via xdotool/ydotool |
| `Both` | Copies to clipboard AND types at cursor |

### TTS settings

```rust
tts_voice: "en_US-lessac-medium".into(),  // Piper voice model
tts_speed: "1.0".into(),                   // 1.0 = normal, 1.5 = faster
```

TTS requires voice-speak to be installed. Without it, screen-ocr works normally but without speech.

### Tesseract language

Edit `python/ocr_extract.py` to change the `lang` parameter:

```python
text = pytesseract.image_to_string(img, lang='eng').strip()       # English (default)
text = pytesseract.image_to_string(img, lang='jpn').strip()       # Japanese
text = pytesseract.image_to_string(img, lang='eng+fra').strip()   # English + French
```

Install additional language packs:

```bash
sudo steamos-readonly disable
sudo pacman -S tesseract-data-jpn tesseract-data-fra
sudo steamos-readonly enable
```

Then re-run `./install.sh` to deploy the updated script.

## Architecture

```
screen-ocr/
├── src/main.rs              Rust binary: hotkey listener, capture, OCR dispatch, clipboard, TTS
├── python/ocr_extract.py    Python OCR script (Tesseract wrapper: image path -> text on stdout)
├── Cargo.toml               Rust dependencies (rdev, tempfile, anyhow, ctrlc, libc, serde, serde_json)
├── requirements.txt         Python dependencies (pytesseract, Pillow)
└── install.sh               Cross-distro installer (apt / pacman, auto-detects display server)
```

**Display server auto-detection:**
- **X11:** `slop` (select region) + `maim` (capture) + `xclip` (clipboard)
- **Wayland:** `slurp` (select region) + `grim` (capture) + `wl-copy` (clipboard)

**Threading model:**
- **Main thread**: state machine + blocking subprocess calls
- **rdev listener thread**: captures raw key events, sends over mpsc channel

**Installed files:**

| Path | Purpose |
|------|---------|
| `~/.local/bin/screen-ocr` | Compiled binary |
| `~/.local/bin/ocr_extract.py` | Python OCR script |
| `~/.local/bin/ocr_extract_wrapper.sh` | Venv-aware wrapper (auto-generated) |
| `~/.local/share/screen-ocr/venv/` | Python virtual environment |
| `~/.local/share/screen-ocr/last_region.json` | Persisted region geometry |

## SteamOS Notes

- Packages installed via pacman (`grim`, `slurp`, `wl-clipboard`, `tesseract`) don't survive SteamOS major updates -- re-run `install.sh` after updates
- User must be in the `input` group for rdev hotkeys: `sudo usermod -aG input $USER` (then reboot)
- Filesystem is read-only by default; `install.sh` handles unlock/re-lock via `steamos-readonly`
- Everything in `~/.local/` (binary, venv, region file) survives updates
- Desktop Mode (KDE Plasma / Wayland) is fully supported; Game Mode (Gamescope) may not forward hotkeys to rdev

## Known Limitations

- Game Mode (Gamescope compositor) may not forward key events to rdev -- use Desktop Mode
- Tesseract OCR works best on high-contrast text with standard fonts
- TTS requires voice-speak to be installed separately
- `DeliveryMode::Type` on Wayland requires `ydotool` and the `ydotoold` daemon
- Configuration changes require editing `src/main.rs` and rebuilding (no config file)
- `slurp` may not appear if a game holds an exclusive fullscreen compositor lock

## License

MIT -- see [LICENSE](../../LICENSE)
