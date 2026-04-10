# Deck Reader (Windows)

Multi-hotkey screen OCR + text-to-speech tool for Windows 10/11. Listens globally for three configurable key combos: one to interactively select a screen region and OCR it, one to instantly re-capture the last saved region, and one to toggle TTS on clipboard text. Designed for reading visual novel dialogue aloud -- select the text box once, then press a single key for each new line. Fully offline: Tesseract for OCR, Piper for TTS, no cloud services or API keys required.

## Platform

- **Windows 10 / Windows 11** -- MVP support (clipboard only, no persistent TTS daemon)

## Prerequisites

- **Python 3.10+** -- [python.org](https://www.python.org/downloads/) (add to PATH during install)
- **Rust / cargo** -- [rustup.rs](https://rustup.rs/)
- **winget** -- included in Windows 10 1809+ / Windows 11 (update via Microsoft Store -> App Installer)

## Installation

From an elevated PowerShell session (right-click -> "Run as Administrator"):

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

## Usage

| Hotkey | Action |
|--------|--------|
| `Alt + U` | Select region -- fullscreen overlay, drag a rectangle, OCR + clipboard |
| `Alt + I` | Re-OCR last saved region -> clipboard |
| `Alt + Y` | Toggle TTS -- speaks clipboard text, or stops if already speaking |

### Visual novel workflow

1. Launch the game (windowed or fullscreen)
2. Start `deck-reader` from the Start Menu or a terminal
3. Press `Alt+U` to draw a rectangle around the dialogue text box
4. Press `Alt+I` each time the dialogue advances (no re-drawing needed)
5. Press `Alt+Y` to stop speech mid-sentence or to read arbitrary clipboard text

### Key detection mode

Discover raw keycodes for button mapping:

```powershell
deck-reader --detect-keys
```

## Configuration

Config file: `%APPDATA%\deck-reader\config.toml`

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
delivery_mode = "clipboard"     # "clipboard" only on Windows (MVP)
cleanup       = true            # clean OCR artifacts (stray symbols, repeated punct)
```

### Hotkey format

- Named keys: `MetaLeft`, `AltLeft`, `KeyA`--`KeyZ`, `F1`--`F12`, `Space`, `Return`, etc.
- Combos: `"Alt+KeyU"`, `"MetaLeft+F9"`

### Changing the Piper voice

Download `.onnx` and `.onnx.json` files from [Piper's HuggingFace repository](https://huggingface.co/rhasspy/piper-voices) to `%LOCALAPPDATA%\deck-reader\models\`, then set `voice` in `config.toml` to the model name (filename without extension).

## File layout

| Path | Purpose |
|------|---------|
| `%LOCALAPPDATA%\deck-reader\bin\deck-reader.exe` | Compiled binary |
| `%LOCALAPPDATA%\deck-reader\bin\ocr_extract_wrapper.bat` | OCR helper wrapper |
| `%LOCALAPPDATA%\deck-reader\venv\` | Python virtual environment |
| `%LOCALAPPDATA%\deck-reader\models\` | Piper ONNX voice models |
| `%LOCALAPPDATA%\deck-reader\python\` | Python scripts |
| `%LOCALAPPDATA%\deck-reader\last_region.json` | Persisted OCR region |
| `%APPDATA%\deck-reader\config.toml` | Configuration |

## Known limitations (MVP)

- `delivery_mode = "clipboard"` only -- text injection (`"type"` / `"both"`) not yet implemented
- Persistent TTS daemon not available -- each TTS request pays a ~3-5 s Piper cold-start delay
- GUI status window not shown -- headless operation only (console output)
- Multi-monitor region selection is limited to the monitor containing the drag start point

## SmartScreen warning

The unsigned binary will trigger Windows Defender SmartScreen on first run.
To bypass: right-click `deck-reader.exe` -> Properties -> check **Unblock** -> OK.
Alternatively, launch from a terminal -- SmartScreen only prompts for GUI launches.

## License

MIT -- see [LICENSE](../../LICENSE)
