# Deck Reader (SteamDeck)

Multi-hotkey screen OCR + text-to-speech tool for SteamDeck Desktop Mode. Listens globally for three configurable key combos: one to interactively select a screen region and OCR it, one to instantly re-capture the last saved region, and one to toggle TTS on highlighted or clipboard text. Designed for reading visual novel dialogue aloud -- select the text box once, then press a single key for each new line. Fully offline: Tesseract for OCR, Piper for TTS, no cloud services or API keys required.

## Platform

- **SteamDeck / SteamOS 3.x** — primary target (KDE Plasma / Wayland)
- **Windows 10 / Windows 11** — MVP support (clipboard only, no persistent TTS daemon)

## Prerequisites

- **Rust 1.70+** on your dev machine (not required on the SteamDeck itself)
- **Python 3** (pre-installed on SteamOS)
- System packages installed via pacman (handled by `install.sh`):
  - `wl-clipboard` (provides `wl-paste`, `wl-copy`)
  - `grim` (Wayland screenshot capture)
  - `slurp` (Wayland interactive region selection)
  - `tesseract` + `tesseract-data-eng` (OCR engine)
  - `tk` (Python GUI for status window)

> **SteamOS read-only filesystem:** The installer temporarily unlocks the filesystem with `steamos-readonly disable`, installs packages, then re-locks it. You do not need to do this manually.

## Installation

### Step 1: Build on your dev machine

On any x86_64 Linux PC with Rust installed:

```bash
cd deck-reader
cargo build --release
```

### Step 2: Copy to SteamDeck

Copy the entire `deck-reader/` folder (including `target/release/deck-reader`) to the SteamDeck:

```bash
scp -r deck-reader/ deck@steamdeck:~/deck-reader/
```

### Step 3: Run the installer

On the SteamDeck, in Desktop Mode:

```bash
cd ~/deck-reader
chmod +x install.sh
./install.sh
```

The installer will:
1. Unlock the SteamOS filesystem and install system packages via pacman
2. Re-lock the filesystem
3. Check/offer to add your user to the `input` group
4. Verify the pre-built binary exists
5. Create a Python venv at `~/.local/share/deck-reader/venv/` and install dependencies
6. Download the Piper voice model `en_US-lessac-medium` (~65 MB)
7. Install files to `~/.local/bin/` and generate wrapper scripts
8. Create a KDE application menu entry

### Step 4: Ensure PATH is set

Add to `~/.bashrc` if not already present:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Step 5: Launch

```bash
deck-reader
```

Or search "Deck Reader" in the KDE application menu.

## Usage

| Hotkey | Action |
|--------|--------|
| `Alt + U` | **Select region** -- draw a rectangle, OCR it, copy to clipboard, speak aloud |
| `Alt + I` | **Re-capture** -- instantly re-OCR the last saved region, speak the new text |
| `Alt + Y` | **TTS toggle** -- speak highlighted/clipboard text, or stop if already speaking |

### Visual novel workflow

1. Launch the game in Desktop Mode (windowed or fullscreen)
2. Start `deck-reader` from the KDE app menu or terminal
3. Press `Alt+U` to draw a rectangle around the dialogue text box
4. Press `Alt+I` each time the dialogue advances (no re-drawing needed)
5. Press `Alt+Y` to stop speech mid-sentence or to read arbitrary selected text

### Key detection mode

Discover raw keycodes for Steam Input button mapping:

```bash
deck-reader --detect-keys
```

Press any button to see its keycode, then use that code in `config.toml`.

## Configuration

Config file: `~/.config/deck-reader/config.toml`

Auto-created with defaults on first run. Edit with any text editor; restart `deck-reader` for changes to take effect.

```toml
[hotkeys]
tts_toggle  = "Alt+KeyY"
ocr_select  = "Alt+KeyU"
ocr_capture = "Alt+KeyI"

[tts]
voice = "en_US-lessac-medium"   # Piper model name (must exist in models dir)
speed = 1.0                     # 1.0=normal, 1.5=faster, 0.8=slower

[ocr]
language      = "eng"           # Tesseract lang codes: "eng", "eng+jpn", etc.
delivery_mode = "clipboard"     # "clipboard" | "type" | "both"
cleanup       = true            # clean OCR artifacts (stray symbols, repeated punct)

[paths]
# Optional overrides (defaults shown):
# models_dir  = "~/.local/share/deck-reader/models"
# venv_dir    = "~/.local/share/deck-reader/venv"
# region_file = "~/.local/share/deck-reader/last_region.json"
```

### Hotkey format

- Named keys: `MetaLeft`, `AltLeft`, `KeyA`--`KeyZ`, `F1`--`F12`, `Space`, `Return`, etc.
- Combos: `"Alt+KeyU"`, `"MetaLeft+F9"`
- Raw keycodes from Steam Input: `"191"` or `"Unknown(191)"`

### Adding Tesseract language packs

```bash
sudo steamos-readonly disable
sudo pacman -S tesseract-data-jpn tesseract-data-fra
sudo steamos-readonly enable
```

Then set `language = "jpn"` or `language = "eng+jpn"` in `config.toml`.

### Changing the Piper voice

Download `.onnx` and `.onnx.json` files from [Piper's HuggingFace repository](https://huggingface.co/rhasspy/piper-voices) to `~/.local/share/deck-reader/models/`, then set `voice` in `config.toml` to the model name (filename without extension).

## Architecture

```
deck-reader/
├── src/main.rs           Rust binary: hotkey listener, state machine, subprocess dispatch
├── python/
│   ├── ocr_extract.py    Tesseract OCR wrapper (image path -> text on stdout)
│   ├── tts_speak.py      Piper TTS synthesis + paplay playback
│   ├── tts_daemon.py     Persistent TTS daemon (Unix socket, faster startup)
│   └── gui_window.py     Status GUI window
├── Cargo.toml            Rust dependencies (rdev, anyhow, toml, serde, serde_json, ...)
├── requirements.txt      Python dependencies (piper-tts, pytesseract, Pillow, ...)
└── install.sh            SteamDeck installer (pacman, venv, model download, menu entry)
```

**Threading model:**
- **Main thread**: state machine + blocking subprocess calls
- **rdev listener thread**: captures raw key events from `/dev/input`, sends over mpsc channel

**Subprocess isolation:** TTS processes are spawned with `setsid()` in their own process group. Stopping playback sends `SIGKILL` to the negative PID, killing the entire group (shell, Python, paplay) instantly.

**Installed files:**

| Path | Purpose |
|------|---------|
| `~/.local/bin/deck-reader` | Compiled binary |
| `~/.local/bin/tts_speak_wrapper.sh` | Venv-aware TTS wrapper (auto-generated) |
| `~/.local/bin/ocr_extract_wrapper.sh` | Venv-aware OCR wrapper (auto-generated) |
| `~/.config/deck-reader/config.toml` | Configuration (auto-created on first run) |
| `~/.local/share/deck-reader/venv/` | Python virtual environment |
| `~/.local/share/deck-reader/models/` | Piper ONNX voice models |
| `~/.local/share/deck-reader/last_region.json` | Persisted OCR region geometry |
| `~/.local/share/applications/deck-reader.desktop` | KDE app menu entry |

## SteamOS Notes

- Packages installed via pacman (`wl-clipboard`, `grim`, `slurp`, `tesseract`) don't survive SteamOS major updates -- re-run `install.sh` after updates
- User must be in the `input` group for rdev hotkeys: `sudo usermod -aG input $USER` (then reboot)
- Filesystem is read-only by default; `install.sh` handles unlock/re-lock via `steamos-readonly`
- Everything in `~/.local/` and `~/.config/` (binary, venv, models, config) survives updates
- The `input` group membership survives updates
- Game Mode (Gamescope) may not forward hotkeys to rdev -- use Desktop Mode for best results

## Known Limitations

- Requires Desktop Mode (KDE Plasma) -- Gamescope in Game Mode may not forward key events to rdev
- Tesseract OCR works best on high-contrast text with standard fonts; low-contrast or stylized fonts may produce poor results
- `delivery_mode = "type"` requires the `ydotoold` daemon to be running (`systemctl --user start ydotoold`)
- Electron apps (Discord, VS Code) do not populate the Wayland PRIMARY selection -- use Ctrl+C first, then Alt+Y
- The binary must be cross-compiled on a dev machine (SteamOS lacks development headers by default)
- `slurp` may not appear if a game holds an exclusive fullscreen compositor lock

---

## Windows Installation

### Prerequisites

- **Python 3.10+** — [python.org](https://www.python.org/downloads/) (add to PATH during install)
- **Rust / cargo** — [rustup.rs](https://rustup.rs/)
- **winget** — included in Windows 10 1809+ / Windows 11 (update via Microsoft Store → App Installer)

### Install

From an elevated PowerShell session (right-click → "Run as Administrator"):

```powershell
cd deck-reader
.\install.bat
```

Or without the wrapper:

```powershell
Set-ExecutionPolicy Bypass -Scope Process
.\install.ps1
```

The installer will:
1. Verify prerequisites (Python, Rust, winget)
2. Install Tesseract OCR via `winget install UB-Mannheim.TesseractOCR`
3. Build the Rust binary (`cargo build --release`)
4. Copy binary + wrapper scripts to `%LOCALAPPDATA%\deck-reader\bin\`
5. Create a Python venv at `%LOCALAPPDATA%\deck-reader\venv\` and install dependencies
6. Copy Python scripts to `%LOCALAPPDATA%\deck-reader\python\`
7. Download the Piper voice model `en_US-lessac-medium` (~65 MB from HuggingFace)
8. Write a default config to `%APPDATA%\deck-reader\config.toml`
9. Create a Start Menu shortcut

### Skip model download

If you already have the model or want to download it manually:

```powershell
.\install.ps1 -SkipModel
```

### Usage (Windows)

| Hotkey | Action |
|--------|--------|
| `Alt + U` | Select region — fullscreen overlay, drag a rectangle, OCR + clipboard |
| `Alt + I` | Re-OCR last saved region → clipboard |
| `Alt + Y` | Toggle TTS — speaks clipboard text, or stops if already speaking |

### Windows limitations (MVP)

- `delivery_mode = "clipboard"` only — text injection (`"type"` / `"both"`) not yet implemented
- Persistent TTS daemon not available — each TTS request pays a ~3–5 s Piper cold-start delay
- GUI status window not shown — headless operation only (console output)
- Multi-monitor region selection is limited to the monitor containing the drag start point

### Windows file layout

| Path | Purpose |
|------|---------|
| `%LOCALAPPDATA%\deck-reader\bin\deck-reader.exe` | Compiled binary |
| `%LOCALAPPDATA%\deck-reader\bin\ocr_extract_wrapper.bat` | OCR helper wrapper |
| `%LOCALAPPDATA%\deck-reader\venv\` | Python virtual environment |
| `%LOCALAPPDATA%\deck-reader\models\` | Piper ONNX voice models |
| `%LOCALAPPDATA%\deck-reader\python\` | Python scripts |
| `%LOCALAPPDATA%\deck-reader\last_region.json` | Persisted OCR region |
| `%APPDATA%\deck-reader\config.toml` | Configuration |

### SmartScreen warning

The unsigned binary will trigger Windows Defender SmartScreen on first run.
To bypass: right-click `deck-reader.exe` → Properties → check **Unblock** → OK.
Alternatively, launch from a terminal — SmartScreen only prompts for GUI launches.

---

## License

MIT -- see [LICENSE](../../LICENSE)
