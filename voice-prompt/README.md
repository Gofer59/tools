# voice-prompt

> Push-to-talk speech-to-text that types in any window. Whisper quality, 250 ms warm-start, configured live from a real GUI.

Hold a hotkey, speak, release — your words appear at the cursor. The Whisper daemon loads once at startup and stays resident, so every subsequent transcription is fast. All settings are changed live from a three-tab GUI; nothing requires a restart or a text editor.

## Features

- Push-to-talk hotkey (fully configurable)
- Live-apply settings — changes take effect in under 1 second, no restart
- 9-model Whisper catalog (tiny through large-v3) with streaming download
- Custom `.onnx` model import
- Persistent Python daemon — no cold-start latency after the first recording
- Cross-platform: Linux X11 and Windows 10/11
- MIT licensed, no telemetry

## Platforms

| Platform | Status |
|---|---|
| Linux X11 | Supported |
| Linux Wayland | Partial — global hotkeys via rdev evdev fallback; requires user in `input` group |
| Windows 10/11 | Supported |
| macOS | Untested |

## Quick install

**Linux:**

```bash
git clone https://github.com/SVaiva/voice-tools
cd voice-tools/voice-prompt
cargo tauri build
./install.sh
```

**Windows (PowerShell):**

```powershell
git clone https://github.com/SVaiva/voice-tools
cd voice-tools/voice-prompt
.\install.ps1 -FromSource
```

## Usage

Launch `voice-prompt`. The window has three tabs:

- **Settings** — configure the hotkey, Whisper model, transcription language, VAD, Python interpreter path, and compute type. Every change is written to disk and applied immediately; the daemon restarts in the background within about one second.
- **Models** — browse the nine-model Whisper catalog (tiny, base, small, medium, large-v1/v2/v3, distil-large-v2, distil-large-v3), download any of them with a live progress bar, or import a custom `.onnx` model from disk.
- **About** — version number, license, and a language switcher for the UI itself.

Day-to-day use: launch `voice-prompt` once, configure the hotkey and model to your taste, then minimize to the system tray. From that point on, hold the hotkey to record, release to transcribe — the text is typed at wherever your cursor is.

## Settings reference

| Field | Default | Effect |
|---|---|---|
| `push_to_talk_key` | `Ctrl+Alt+Space` | Hold to record, release to transcribe and type |
| `whisper_model` | `small` | Whisper model size used for transcription |
| `language` | `en` | Transcription language (`en`, `fr`, `auto`) |
| `vad_filter` | `true` | Voice Activity Detection — strips leading and trailing silence |
| `python_bin` | `python3` | Path to the Python interpreter that runs the daemon |
| `compute_type` | `int8` | Whisper quantization (`int8`, `float16`, `float32`) |

Config is stored as JSON:

- **Linux:** `~/.local/share/voice-prompt/config.json`
- **Windows:** `%LOCALAPPDATA%\voice-prompt\config.json`

## Models

Catalog models are downloaded on demand to:

- **Linux:** `~/.local/share/voice-prompt/models/`
- **Windows:** `%LOCALAPPDATA%\voice-prompt\models\`

**Custom models:** click "Add custom model" in the Models tab and select your `.onnx` file. A sibling `.onnx.json` config file must be present in the same directory — this is the standard `faster-whisper` config format.

Model sizes for reference:

| Model | Size on disk | Relative speed |
|---|---|---|
| tiny | ~75 MB | fastest |
| base | ~145 MB | fast |
| small | ~465 MB | good balance (default) |
| medium | ~1.5 GB | accurate |
| large-v3 | ~3.1 GB | most accurate |

## Troubleshooting

**Hotkey not working on Wayland:**
Global shortcuts are registered via tauri-plugin-global-shortcut. On Wayland this falls back to rdev evdev. Ensure your user is in the `input` group:

```bash
sudo gpasswd -a $USER input
```

Log out and back in for the group change to take effect.

**No microphone audio:**
Check available sources with `pactl list sources short`. If you are using ALSA directly, verify sample-rate compatibility. PipeWire users: ensure `pipewire-alsa` is installed.

**Python venv not found:**
Re-run `./install.sh`. Or set up manually:

```bash
python3 -m venv ~/.local/share/voice-prompt/venv
~/.local/share/voice-prompt/venv/bin/pip install faster-whisper
```

**Model download fails:**
HuggingFace may be temporarily unavailable. Wait a few minutes and retry. Verify your network can reach `huggingface.co`.

**Windows: antivirus blocks unsigned MSI:**
Allowlist the installer, or build from source using the `-FromSource` flag described above.

**Windows: missing VC++ Redistributable or WebView2:**
The installer handles WebView2 automatically. For the VC++ Redistributable, download it from Microsoft's official site.

**`libxdo` not found (Linux build error):**
Install the development headers:

```bash
# Debian / Ubuntu
sudo apt install libxdo-dev

# Fedora / RHEL
sudo dnf install libxdo-devel
```

`install.sh` handles this automatically when run on a supported distribution.

## Build from source

Prerequisites: Rust (via rustup), Node.js 20+, and `cargo-tauri`.

```bash
git clone https://github.com/SVaiva/voice-tools
cd voice-tools/voice-prompt

# Install UI dependencies
cd ui && npm install && cd ..

# Run in development mode (hot-reload)
cd src-tauri && cargo tauri dev

# Build a release binary
cargo tauri build
```

The release bundle is placed under `src-tauri/target/release/bundle/`.

## Contributing

Small personal project — PRs are welcome. Please run `cargo fmt && cargo clippy` before submitting. Open an issue first for larger changes so the direction can be agreed on before you invest the time.

## License

MIT — see [LICENSE](../LICENSE).
