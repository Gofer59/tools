# screen-ocr

Hotkey-triggered screen region OCR with text-to-speech for Linux. Designed for visual novels on SteamDeck: select the dialogue text box once, then press a single key for each new line to OCR it, copy to clipboard, and hear it read aloud. Fully offline -- no cloud services, no API keys.

## How it works

```
F10  -->  Draw selection rectangle (saved to disk)
F9   -->  Instant re-capture of saved region  -->  Tesseract OCR  -->  Clipboard  -->  TTS
```

```
Rust binary (screen-ocr)
  |-- rdev             global hotkey listener (F9 / F10)
  |-- slop / slurp     interactive region selection (X11 / Wayland)
  |-- maim / grim      screen region capture (X11 / Wayland)
  |-- tempfile         auto-cleanup temporary PNG
  |-- serde_json       persist region geometry to ~/.local/share/screen-ocr/
  |-- subprocess --> python3 ocr_extract.py <image>
  |                    '-- pytesseract (Tesseract OCR, offline, CPU)
  |-- xclip / wl-copy  clipboard delivery
  '-- subprocess --> tts_speak_wrapper.sh <text> <voice> <speed>
                       '-- Piper TTS (ONNX, CPU, offline) --> paplay
```

**Visual novel workflow:**
1. Press **F10** to draw a rectangle around the dialogue text box (done once)
2. Press **F9** each time the dialogue advances -- the saved region is re-captured instantly
3. Text is extracted, copied to clipboard, and spoken aloud automatically
4. Press **F10** again if the text box moves or you switch games

The selected region geometry is saved to `~/.local/share/screen-ocr/last_region.json` and persists across restarts.

---

## Installation

### Prerequisites

- **Rust** (1.70+) -- install via [rustup](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Python 3.10+** -- pre-installed on most Linux distributions
- **voice-speak** (optional, for TTS) -- install from the sibling `voice-speak/` project:
  ```bash
  cd ../voice-speak && ./install.sh
  ```

### Linux (Debian / Ubuntu / Linux Mint)

```bash
git clone <this-repo> && cd screen-ocr
chmod +x install.sh
./install.sh
```

The installer automatically handles everything:

1. Detects your display server (X11 or Wayland)
2. Installs system packages via `apt`:
   - **X11:** `maim`, `slop`, `xclip`, `xdotool`, `tesseract-ocr`, `libasound2-dev`
   - **Wayland:** `grim`, `slurp`, `wl-clipboard`, `tesseract-ocr`, `libasound2-dev`
3. Checks for voice-speak TTS (optional)
4. Creates a Python virtual environment at `~/.local/share/screen-ocr/venv/`
5. Installs Python dependencies: `pytesseract`, `Pillow`
6. Builds the Rust binary in release mode
7. Installs everything to `~/.local/bin/`

### SteamDeck (SteamOS)

SteamOS is Arch-based and uses an immutable (read-only) root filesystem by default. You need to unlock it before installing system packages.

#### Step 1: Unlock the filesystem

```bash
sudo steamos-readonly disable
```

> **Note:** SteamOS updates may re-enable the read-only filesystem, requiring you to run this again after updates.

#### Step 2: Initialize pacman keyring (first time only)

```bash
sudo pacman-key --init
sudo pacman-key --populate archlinux
sudo pacman-key --populate holo
```

#### Step 3: Install system dependencies

SteamDeck Desktop Mode runs KDE Plasma on Wayland, so you need the Wayland tools:

```bash
sudo pacman -S --noconfirm tesseract tesseract-data-eng grim slurp wl-clipboard alsa-lib python
```

For Japanese visual novels (or other languages):

```bash
sudo pacman -S tesseract-data-jpn    # Japanese
sudo pacman -S tesseract-data-chi_sim  # Simplified Chinese
```

If you also want the `Type` delivery mode (typing text at cursor on Wayland):

```bash
sudo pacman -S --noconfirm ydotool
sudo systemctl enable --now ydotoold
```

#### Step 4: Install Rust (if not already installed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

#### Step 5: Install voice-speak for TTS (recommended)

```bash
cd voice-speak
chmod +x install.sh
./install.sh
```

#### Step 6: Build and install screen-ocr

```bash
cd screen-ocr
chmod +x install.sh
./install.sh
```

The installer detects `pacman` and adapts accordingly.

#### Step 7: Ensure PATH is set

Add to your `~/.bashrc` if not already present:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

#### SteamDeck limitations

- **Desktop Mode** (KDE Plasma / Wayland): fully supported. Uses `grim` + `slurp` for capture, `wl-copy` for clipboard, Piper for TTS.
- **Game Mode** (Gamescope compositor): the global hotkey listener (`rdev`) may not receive key events through Gamescope. Use Desktop Mode for best results. You can add games to Steam as non-Steam games and run them in Desktop Mode.

---

## Usage

```bash
screen-ocr          # runs in foreground, Ctrl-C to quit
```

1. Press **F10** to draw a selection rectangle around the text area
2. The region is saved and OCR runs immediately -- text appears in clipboard and is spoken aloud
3. Press **F9** to re-capture the same region instantly (no drawing needed)
4. Press **F9** again each time the dialogue advances
5. Press **F10** to select a different region at any time

On startup, `screen-ocr` prints its active configuration:

```
+============================================+
|         screen-ocr  ready                   |
+============================================+
|  F9:       Quick capture (re-use region)    |
|  F10:      Select new region                |
|  Display:  X11                              |
|  Capture:  slop + maim -g                   |
|  Clipboard:xclip                            |
|  TTS:      Piper (voice-speak)              |
|  Region:   loaded from disk                 |
|                                             |
|  F10 -> draw region -> OCR -> clipboard     |
|  F9  -> instant re-capture -> OCR -> TTS    |
|  Ctrl-C to quit                             |
+============================================+
```

### Cancel a selection

Press **Escape** while the crosshair is visible (during F10) to cancel and return to idle.

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

---

## Configuration

All configuration is done by editing `src/main.rs` in the `Config` struct, then rebuilding with `./install.sh`.

### Hotkeys

Edit `quick_capture_key` and `select_region_key` in `Config::default()`:

```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            quick_capture_key: Key::F9,     // re-capture stored region
            select_region_key: Key::F10,    // interactive selection
            // ...
        }
    }
}
```

#### Available keys

| Value | Physical key |
|-------|-------------|
| `Key::F1` .. `Key::F12` | Function keys |
| `Key::KeyA` .. `Key::KeyZ` | A-Z letter keys |
| `Key::Num0` .. `Key::Num9` | Number keys |
| `Key::Space` | Space bar |
| `Key::PrintScreen` | Print Screen |
| `Key::MetaLeft` / `Key::MetaRight` | Super/Meta/Windows keys |
| `Key::Alt` / `Key::AltGr` | Alt keys |
| `Key::ControlLeft` / `Key::ControlRight` | Ctrl keys |

#### Examples

| Quick Capture | Select Region | Configuration |
|--------------|--------------|---------------|
| F9 | F10 | Default |
| F7 | F8 | `quick_capture_key: Key::F7, select_region_key: Key::F8` |
| Home | End | `quick_capture_key: Key::Home, select_region_key: Key::End` |
| PrintScreen | Pause | `quick_capture_key: Key::PrintScreen, select_region_key: Key::Pause` |

### Delivery mode

Controls what happens with the extracted text. Edit `delivery_mode` in `Config::default()`:

```rust
delivery_mode: DeliveryMode::Clipboard,
```

| Mode | Behavior | Tools used |
|------|----------|-----------|
| `DeliveryMode::Clipboard` | Copies text to system clipboard (default) | `xclip` (X11) / `wl-copy` (Wayland) |
| `DeliveryMode::Type` | Types text at the current cursor position | `xdotool` (X11) / `ydotool` (Wayland) |
| `DeliveryMode::Both` | Copies to clipboard AND types at cursor | Both of the above |

`Clipboard` is the default because OCR output often needs minor corrections before use.

### TTS (text-to-speech)

TTS requires the `voice-speak` tool to be installed (provides Piper TTS + paplay audio). If not installed, screen-ocr works normally but without speech.

Edit TTS settings in `Config::default()`:

```rust
tts_wrapper: PathBuf::from(".../.local/bin/tts_speak_wrapper.sh"),
tts_voice: "en_US-lessac-medium".into(),  // Piper voice model
tts_speed: "1.0".into(),                   // 1.0 = normal, 1.5 = faster, 0.8 = slower
```

| Setting | Default | Description |
|---------|---------|-------------|
| `tts_wrapper` | `~/.local/bin/tts_speak_wrapper.sh` | Path to the voice-speak TTS wrapper |
| `tts_voice` | `en_US-lessac-medium` | Piper voice model name |
| `tts_speed` | `1.0` | Speed multiplier (1.0 = normal) |

TTS is non-blocking: the speech plays in the background while you continue interacting with the game. If you press F9 again before the previous speech finishes, it is interrupted immediately and the new text starts playing.

### Region geometry persistence

The selected region is stored as JSON at:

```
~/.local/share/screen-ocr/last_region.json
```

Format:
```json
{
  "x": 100,
  "y": 500,
  "w": 800,
  "h": 200
}
```

This file persists across restarts. Delete it to force re-selection on next F9 press.

### Tesseract language

By default, Tesseract uses English. To add other languages:

#### Install language packs

```bash
# Debian/Ubuntu/Mint
sudo apt install tesseract-ocr-fra   # French
sudo apt install tesseract-ocr-deu   # German
sudo apt install tesseract-ocr-jpn   # Japanese
sudo apt install tesseract-ocr-chi-sim  # Simplified Chinese

# Arch/SteamOS
sudo pacman -S tesseract-data-fra
sudo pacman -S tesseract-data-deu
sudo pacman -S tesseract-data-jpn
sudo pacman -S tesseract-data-chi_sim
```

#### Configure the language in the Python script

Edit `python/ocr_extract.py` and change the `image_to_string` call:

```python
# Single language
text = pytesseract.image_to_string(img, lang='fra').strip()

# Multiple languages (Tesseract tries all, picks best match)
text = pytesseract.image_to_string(img, lang='eng+fra+deu').strip()

# Japanese visual novels
text = pytesseract.image_to_string(img, lang='jpn').strip()
```

Then re-run `./install.sh` to deploy the updated script.

#### List installed languages

```bash
tesseract --list-langs
```

### OCR engine

The default engine is Tesseract via `pytesseract`. To swap it for another engine, edit `python/ocr_extract.py`. The contract is simple:

- **Input:** image path as `sys.argv[1]`
- **Output:** extracted text printed to `stdout`
- **Diagnostics:** print to `stderr`

For example, to use EasyOCR instead:

```python
import sys
import easyocr

reader = easyocr.Reader(['en'], gpu=False)
results = reader.readtext(sys.argv[1], detail=0)
print('\n'.join(results))
```

Add `easyocr` to `requirements.txt` and re-run `./install.sh`.

---

## File layout

```
screen-ocr/
|-- src/main.rs              Rust binary (hotkeys, capture, OCR, clipboard, TTS)
|-- Cargo.toml               Rust dependencies
|-- python/ocr_extract.py    Python OCR script (Tesseract wrapper)
|-- requirements.txt         Python dependencies (pytesseract, Pillow)
|-- install.sh               Cross-distro installer (apt / pacman)
'-- README.md                This file
```

After installation:

```
~/.local/bin/
|-- screen-ocr               Compiled binary
|-- ocr_extract.py            Python OCR script
'-- ocr_extract_wrapper.sh    Venv-aware wrapper (auto-generated)

~/.local/share/screen-ocr/
|-- venv/                     Python virtual environment
'-- last_region.json          Saved region geometry (created on first F10)
```

---

## Troubleshooting

### "Failed to run slop" / "Failed to run slurp"

The region selection tool for your display server is not installed:

```bash
# X11
sudo apt install slop           # Debian/Ubuntu/Mint
sudo pacman -S slop             # Arch/SteamOS

# Wayland
sudo apt install slurp          # Debian/Ubuntu
sudo pacman -S slurp            # Arch/SteamOS
```

### "Failed to run maim" / "Failed to run grim"

The screen capture tool is not installed:

```bash
# X11
sudo apt install maim           # Debian/Ubuntu/Mint
sudo pacman -S maim             # Arch/SteamOS

# Wayland
sudo apt install grim           # Debian/Ubuntu
sudo pacman -S grim             # Arch/SteamOS
```

### "No saved region" on F9

No region has been selected yet. Press F10 first to draw a selection rectangle. The region is then saved and F9 will work.

### "Failed to run OCR script" / "Did you run install.sh?"

The Python venv or wrapper script is missing. Re-run `./install.sh`.

### "TTS error (continuing without speech)"

The voice-speak TTS wrapper is not installed. Install it:

```bash
cd ../voice-speak && ./install.sh
```

screen-ocr will continue to work without TTS -- OCR and clipboard still function normally.

### Empty or garbled OCR output

- Ensure you selected a region with readable text (not just images/icons).
- Tesseract works best on high-contrast text with standard fonts.
- For very small text, try selecting a larger area or zooming in first.
- Check if the correct language pack is installed: `tesseract --list-langs`
- For Japanese text, install `tesseract-data-jpn` and set `lang='jpn'` in the Python script.

### "rdev error" on Wayland

The `rdev` crate uses X11 for key event listening. On pure Wayland, it requires XWayland to be running. KDE Plasma on SteamDeck Desktop Mode runs XWayland by default, so this should work out of the box. If you see this error, ensure XWayland is enabled in your compositor settings.

### SteamOS: "error: could not open file ... Permission denied"

The filesystem is read-only. Run:

```bash
sudo steamos-readonly disable
```

### xdotool / ydotool not typing text

- **X11:** Ensure `xdotool` is installed: `sudo apt install xdotool`
- **Wayland (SteamDeck):** Ensure `ydotool` is installed and its daemon is running:
  ```bash
  sudo pacman -S ydotool
  sudo systemctl enable --now ydotoold
  ```

---

## Dependencies

### Rust crates

| Crate | Version | Purpose |
|-------|---------|---------|
| `rdev` | 0.5 | Global keyboard event listener |
| `tempfile` | 3 | Auto-cleanup temporary files |
| `anyhow` | 1 | Ergonomic error handling |
| `ctrlc` | 3 | Graceful Ctrl-C shutdown |
| `libc` | 0.2 | Process group management (setsid, kill) for TTS |
| `serde` | 1 | Region struct serialization |
| `serde_json` | 1 | Region geometry JSON persistence |

### System packages

| Package | X11 | Wayland | Purpose |
|---------|-----|---------|---------|
| `maim` | Required | -- | Screen region capture |
| `slop` | Required | -- | Interactive region selection |
| `grim` | -- | Required | Screenshot capture |
| `slurp` | -- | Required | Interactive region selection |
| `xclip` | Required | -- | Clipboard access |
| `wl-clipboard` | -- | Required | Clipboard access |
| `xdotool` | Optional | -- | Type text at cursor |
| `ydotool` | -- | Optional | Type text at cursor |
| `tesseract-ocr` | Required | Required | OCR engine |
| `libasound2-dev` / `alsa-lib` | Required | Required | ALSA headers (build dependency for rdev) |

### Python packages

| Package | Purpose |
|---------|---------|
| `pytesseract` | Python wrapper for Tesseract CLI |
| `Pillow` | Image loading (PIL) |

### Optional: voice-speak (TTS)

| Component | Location | Purpose |
|-----------|----------|---------|
| `tts_speak_wrapper.sh` | `~/.local/bin/` | Venv-aware TTS entry point |
| `tts_speak.py` | `~/.local/bin/` | Piper TTS Python script |
| Piper model | `~/.local/share/voice-speak/models/` | ONNX voice model |
| Python venv | `~/.local/share/voice-speak/venv/` | Piper + dependencies |
