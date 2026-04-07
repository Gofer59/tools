# voice-speak

Text-to-speech for highlighted text — press a hotkey, hear it spoken aloud.

```
Select any text → press Right Alt → audio plays through your speakers
Press Right Alt again while speaking → stops immediately
```

Works in any application on Linux (terminals, browsers, PDFs, code editors…).

## Architecture

```
Rust binary (voice-speak)
  ├─ rdev          — global hotkey listener (toggle: speak / stop)
  ├─ xclip         — reads PRIMARY selection (highlighted text), falls back to CLIPBOARD
  └─ subprocess  → tts_speak_wrapper.sh  (activates Python venv)
                     └─ python3 tts_speak.py <text> <voice> <speed>
                          ├─ piper-tts   (ONNX neural TTS, CPU, fully offline)
                          └─ paplay      (routes through PulseAudio/PipeWire)
```

The subprocess is spawned in its own process group (`setsid`), so pressing the hotkey to stop sends `SIGKILL` to the entire group — Python and `paplay` both die instantly.

## Requirements

| Tool | Install |
|------|---------|
| Rust + Cargo | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| xclip | `sudo apt install xclip` (X11) |
| wl-clipboard | `sudo apt install wl-clipboard` (Wayland) |
| python3 | pre-installed on Linux Mint |
| paplay | pre-installed (part of `pulseaudio-utils`) |

Python dependencies (`piper-tts`, `sounddevice`, `numpy`) are installed automatically into an isolated venv by `install.sh`.

## Install

```bash
chmod +x install.sh
./install.sh
```

This will:
1. Build the Rust binary in release mode
2. Create a Python venv at `~/.local/share/voice-speak/venv/`
3. Download the default Piper voice model (`en_US-lessac-medium`) to `~/.local/share/voice-speak/models/`
4. Copy everything to `~/.local/bin/`

## Usage

```bash
voice-speak          # starts listening; Ctrl-C to quit
```

1. Select / highlight any text in any application
2. Press **Right Alt**
3. The text is spoken aloud (~0.5–1 s latency for short sentences)
4. To stop mid-speech: press **Right Alt** again

## Customisation

All tunable settings are in `src/main.rs` inside `Config::default()`. After any change, re-run `./install.sh` to rebuild and reinstall.

### Change the hotkey

```rust
hotkey: Key::AltGr,        // Right Alt (default)
hotkey: Key::F10,          // F10
hotkey: Key::ControlRight, // Right Ctrl
```

Available key names: https://docs.rs/rdev/latest/rdev/enum.Key.html

### Change the voice

```rust
voice: "en_US-lessac-medium".into(),   // default
voice: "en_US-ryan-medium".into(),     // more expressive male
voice: "en_GB-alan-medium".into(),     // British male
```

The model must be downloaded first (see **Voices** below). No code change is required if you just want to test a voice — you can call the Python script directly:

```bash
~/.local/share/voice-speak/venv/bin/python3 \
  ~/.local/bin/tts_speak.py "Hello world" en_US-ryan-medium 1.0
```

### Change the speech speed

```rust
speed: 1.0,    // normal (default)
speed: 1.5,    // 50% faster
speed: 0.8,    // 20% slower
```

### Advanced synthesis parameters (`tts_speak.py`)

Edit `SynthesisConfig` in `tts_speak.py` (no rebuild needed, just copy the file to `~/.local/bin/`):

```python
syn_config = SynthesisConfig(
    length_scale=1.0 / speed,   # phoneme duration; <1 = faster, >1 = slower
    noise_scale=0.667,           # expressiveness / naturalness (0 = robotic, 1 = varied)
    noise_w_scale=0.8,           # rhythm variation (0 = flat/uniform, 1 = natural)
)
```

**About pauses** (commas, periods, etc.): Piper generates pauses based on punctuation internally — there is no dedicated pause parameter. To shorten pauses, increase `speed` or lower `noise_w_scale` toward 0 for more uniform timing.

## Voices

Piper provides dozens of free, offline neural voices. Browse the full list at:
https://huggingface.co/rhasspy/piper-voices/tree/v1.0.0

### Recommended English voices

| Model name | Style | Size |
|---|---|---|
| `en_US-lessac-medium` | neutral male (default) | ~60 MB |
| `en_US-ryan-medium` | expressive male | ~60 MB |
| `en_US-ljspeech-medium` | clear female | ~60 MB |
| `en_GB-alan-medium` | British male | ~60 MB |
| `en_GB-jenny_dioco-medium` | British female | ~60 MB |
| `en_US-lessac-high` | neutral male, higher quality | ~130 MB |

### Downloading a new voice

Each voice needs two files: `.onnx` (model) and `.onnx.json` (config).

```bash
# Example: en_US-ryan-medium
BASE="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/ryan/medium"
MODEL_DIR="$HOME/.local/share/voice-speak/models"

curl -L -o "$MODEL_DIR/en_US-ryan-medium.onnx"      "$BASE/en_US-ryan-medium.onnx"
curl -L -o "$MODEL_DIR/en_US-ryan-medium.onnx.json" "$BASE/en_US-ryan-medium.onnx.json"
```

URL pattern: `.../piper-voices/resolve/v1.0.0/<lang>/<lang_region>/<name>/<quality>/<filename>`

### French voices

```bash
BASE="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/fr/fr_FR/siwis/medium"
curl -L -o "$MODEL_DIR/fr_FR-siwis-medium.onnx"      "$BASE/fr_FR-siwis-medium.onnx"
curl -L -o "$MODEL_DIR/fr_FR-siwis-medium.onnx.json" "$BASE/fr_FR-siwis-medium.onnx.json"
```

Then set `voice: "fr_FR-siwis-medium".into()` in `src/main.rs`.

## File layout

```
voice-speak/
├── Cargo.toml            # rdev, anyhow, ctrlc, libc
├── src/main.rs           # Rust binary: hotkey loop, clipboard read, subprocess mgmt
├── tts_speak.py          # Python: Piper synthesis + paplay playback
├── requirements.txt      # piper-tts, sounddevice, numpy
└── install.sh            # one-shot build + install script

Installed to:
~/.local/bin/
  voice-speak             # Rust binary
  tts_speak.py            # Python TTS script
  tts_speak_wrapper.sh    # venv-activating wrapper (called by Rust)

~/.local/share/voice-speak/
  venv/                   # Python virtual environment
  models/                 # Piper .onnx voice models
```

## Autostart on login

Create `~/.config/autostart/voice-speak.desktop`:

```ini
[Desktop Entry]
Type=Application
Name=voice-speak
Exec=voice-speak
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
```

## Troubleshooting

**No sound plays**
→ Audio goes through `paplay` (PulseAudio/PipeWire). Check your default sink:
→ `pactl info | grep "Default Sink"`
→ Raw ALSA (`sounddevice` direct) is bypassed intentionally — it silently routes to the wrong device on many laptops.

**`xclip: command not found`**
→ `sudo apt install xclip`

**`ERROR: No Piper model found`**
→ The model files are missing from `~/.local/share/voice-speak/models/`. Re-run `./install.sh` or download manually (see **Voices** above).

**rdev requires elevated permissions**
→ `sudo voice-speak` (temporary), or add yourself to the `input` group:
→ `sudo usermod -aG input $USER` then log out and back in.

**Wayland / pure Wayland windows**
→ X11 clipboard is read via `xclip`. For Wayland, the binary auto-detects `WAYLAND_DISPLAY` / `XDG_SESSION_TYPE` and uses `wl-paste` instead.
