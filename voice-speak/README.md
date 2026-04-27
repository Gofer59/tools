# voice-speak

> Highlight any text, hit your hotkey, hear it spoken. 23 Piper voices, 12 language families, fully offline, configured live from a real GUI.

## Features

- Hotkey-triggered TTS — reads selected text aloud (PRIMARY selection → CLIPBOARD fallback on Linux)
- 23 Piper voices across 12 languages: English (US/GB), French, German, Spanish, Italian, Chinese, Portuguese, Russian, Dutch, Polish, Swedish, Turkish, Ukrainian
- Live-apply settings — voice, speed, noise parameters change instantly, no restart required
- Custom `.onnx` model import — add any Piper-compatible voice from the Models tab
- Persistent Python daemon — Piper loads once at startup; no per-utterance startup delay
- Stop playback by pressing the hotkey again (toggle)
- Fully offline — no internet required after model download
- Cross-platform: Linux X11 + Windows 10/11
- MIT licensed, no telemetry

## Platform support

| Platform | Status |
|---|---|
| Linux X11 | ✅ Supported |
| Linux Wayland | ⚠️ Partial — global hotkeys via rdev evdev fallback |
| Windows 10/11 | ✅ Supported |
| macOS | ❌ Untested |

## Quick install

**Linux:**
```bash
git clone https://github.com/SVaiva/voice-tools
cd voice-tools/voice-speak
cargo tauri build
./install.sh
```

**Windows:**
```powershell
git clone https://github.com/SVaiva/voice-tools
cd voice-tools/voice-speak
.\install.ps1 -FromSource
```

## Usage

- **Settings tab**: configure hotkey, voice, speed slider, noise parameters, Python path. All changes live-apply.
- **Models tab**: browse the 23-voice catalog organized by language, download voices, add custom `.onnx` voices.
- **About tab**: version, license, language switcher.

Day-to-day: launch `voice-speak`, configure once, minimize to tray. Select text in any app, press the hotkey to hear it read aloud. Press again to stop.

## Settings reference

| Field | Default | Effect |
|---|---|---|
| `hotkey` | `Ctrl+Alt+V` | Hotkey to start/stop TTS |
| `voice` | `en_US-lessac-medium` | Piper voice model ID |
| `speed` | `1.0` | Playback speed (0.5–2.0) |
| `noise_scale` | `0.667` | Phoneme variation (0.0–1.0) |
| `noise_w_scale` | `0.8` | Duration variation (0.0–1.5) |
| `python_bin` | `python3` | Path to Python interpreter |

Config is stored as JSON at `~/.local/share/voice-speak/config.json` on Linux and `%LOCALAPPDATA%\voice-speak\config.json` on Windows.

## Models

Piper voices download to `~/.local/share/voice-speak/models/` on Linux, `%LOCALAPPDATA%\voice-speak\models\` on Windows. Each voice consists of an `.onnx` file plus a sibling `.onnx.json` config file.

Custom voices: click **Add custom model** in the Models tab — both the `.onnx` and `.onnx.json` files are required.

## Troubleshooting

**Hotkey not working on Wayland:**
rdev requires access to `/dev/input` device files. Add yourself to the `input` group:
```bash
sudo gpasswd -a $USER input
# then log out and back in
```

**No audio output:**
Check available sinks: `pactl list sinks short`. Ensure rodio can open the default sink. If using PipeWire, ensure `pipewire-pulse` is running.

**xclip / wl-paste not found:**
Install via package manager:
```bash
sudo apt install xclip wl-clipboard
```
`install.sh` handles this automatically.

**Voice download fails:**
HuggingFace may be temporarily unavailable. Retry after a moment. Ensure your network can reach `huggingface.co`.

**Custom voice missing .onnx.json:**
Piper requires both the `.onnx` model file AND its sibling `.onnx.json` config. Download both files from the [Piper voices repository](https://github.com/rhasspy/piper/blob/master/VOICES.md).

**Python venv not found:**
Re-run `./install.sh`. Or manually:
```bash
python3 -m venv ~/.local/share/voice-speak/venv
~/.local/share/voice-speak/venv/bin/pip install piper-tts numpy
```

## Build from source

```bash
git clone https://github.com/SVaiva/voice-tools
cd voice-tools/voice-speak
cd ui && npm install && cd ..
cd src-tauri && cargo tauri dev
# or for a release build:
cargo tauri build
```

## Contributing

PRs welcome. Run `cargo fmt && cargo clippy` before submitting.

## License

MIT — see [LICENSE](../LICENSE) file.
