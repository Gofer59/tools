# Voice Speak (SteamDeck)

Text-to-speech for highlighted text on SteamDeck. Press a hotkey to hear any selected text spoken aloud through your speakers; press again to stop immediately. Uses Piper TTS (ONNX neural voices, fully offline) and routes audio through PulseAudio/PipeWire via paplay. Reads the Wayland PRIMARY selection (highlighted text) automatically -- no Ctrl+C needed. Falls back to CLIPBOARD if PRIMARY is empty.

## Platform

SteamDeck (SteamOS 3.x / KDE Plasma / Wayland)

Also works on standard Linux desktops (Debian, Ubuntu, Arch) under Wayland.

## Prerequisites

- **Rust 1.70+** and **Cargo** (for building, or cross-compile on another machine)
- **Python 3** (pre-installed on SteamOS)
- **wl-clipboard** (provides `wl-paste` for reading highlighted text on Wayland)
- **paplay** (pre-installed on SteamOS as part of PipeWire compatibility)

> **SteamOS read-only filesystem:** `wl-clipboard` must be installed via pacman, which requires temporarily unlocking the filesystem:
> ```bash
> sudo steamos-readonly disable
> sudo pacman -S wl-clipboard
> sudo steamos-readonly enable
> ```

Python dependencies (`piper-tts`, `sounddevice`, `numpy`) are installed automatically into an isolated venv by `install.sh`.

## Installation

```bash
cd voice-speak
chmod +x install.sh
./install.sh
```

The installer will:
1. Check system dependencies (python3, wl-paste)
2. Check `input` group membership (required for rdev hotkeys)
3. Create a Python venv at `~/.local/share/voice-speak/venv/`
4. Install Python dependencies (piper-tts, sounddevice, numpy)
5. Download the default Piper voice model `en_US-lessac-medium` (~60 MB) to `~/.local/share/voice-speak/models/`
6. Build the Rust binary (or use a pre-built binary on SteamOS)
7. Install everything to `~/.local/bin/`

Ensure `~/.local/bin` is in your PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Cross-compiling for SteamDeck

If SteamOS lacks development headers for building, compile on any x86_64 Linux machine:

```bash
cargo build --release
scp target/release/voice-speak deck@steamdeck:~/voice-speak/target/release/
```

Then run `./install.sh` on the SteamDeck -- it detects the pre-built binary and skips `cargo build`.

Alternatively, use distrobox (ships with SteamOS):

```bash
distrobox create --name archlinux --image archlinux:latest
distrobox enter archlinux
sudo pacman -S rust base-devel alsa-lib
cd ~/voice-speak && cargo build --release
exit
```

## Usage

```bash
voice-speak          # starts listening; Ctrl-C to quit
```

| Action | Key |
|--------|-----|
| **Speak highlighted text** | Press **Right Ctrl** |
| **Stop playback** | Press **Right Ctrl** again while speaking |

1. Select / highlight any text in any application
2. Press **Right Ctrl** -- text is spoken aloud (~0.5--1 s latency)
3. To stop mid-speech: press **Right Ctrl** again

Text retrieval uses `wl-paste --primary` (whatever is currently highlighted, no Ctrl+C needed). If PRIMARY is empty, it falls back to `wl-paste` (CLIPBOARD).

## Configuration

All configuration is done by editing `src/main.rs` in the `Config` struct, then rebuilding with `./install.sh`.

### Hotkey

```rust
hotkey: Key::ControlRight,  // Right Ctrl (default)
hotkey: Key::AltGr,         // Right Alt
hotkey: Key::F10,           // F10
hotkey: Key::F13,           // useful for Steam Deck back paddle mapping
```

Available key names: https://docs.rs/rdev/latest/rdev/enum.Key.html

### Voice

```rust
voice: "en_US-lessac-medium".into(),    // neutral male (default)
voice: "en_US-ryan-medium".into(),      // expressive male
voice: "en_GB-alan-medium".into(),      // British male
voice: "fr_FR-siwis-medium".into(),     // French female
```

The model must be downloaded first (see **Downloading a new voice** below).

### Speed

```rust
speed: 1.0,    // normal (default)
speed: 1.5,    // 50% faster
speed: 0.8,    // 20% slower
```

### Downloading a new voice

Each voice needs two files: `.onnx` (model) and `.onnx.json` (config). Download from [Piper's HuggingFace repository](https://huggingface.co/rhasspy/piper-voices/tree/v1.0.0):

```bash
BASE="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/ryan/medium"
MODEL_DIR="$HOME/.local/share/voice-speak/models"

curl -L -o "$MODEL_DIR/en_US-ryan-medium.onnx"      "$BASE/en_US-ryan-medium.onnx"
curl -L -o "$MODEL_DIR/en_US-ryan-medium.onnx.json" "$BASE/en_US-ryan-medium.onnx.json"
```

French voice example:

```bash
BASE="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/fr/fr_FR/siwis/medium"
curl -L -o "$MODEL_DIR/fr_FR-siwis-medium.onnx"      "$BASE/fr_FR-siwis-medium.onnx"
curl -L -o "$MODEL_DIR/fr_FR-siwis-medium.onnx.json" "$BASE/fr_FR-siwis-medium.onnx.json"
```

Then set `voice: "fr_FR-siwis-medium".into()` in `src/main.rs` and rebuild.

### Advanced synthesis parameters

Edit `SynthesisConfig` in `tts_speak.py` (no rebuild needed):

```python
syn_config = SynthesisConfig(
    length_scale=1.0 / speed,   # phoneme duration; <1 = faster, >1 = slower
    noise_scale=0.667,           # expressiveness (0 = robotic, 1 = varied)
    noise_w_scale=0.8,           # rhythm variation (0 = flat, 1 = natural)
)
```

## Architecture

```
voice-speak/
├── src/main.rs           Rust binary: hotkey loop, clipboard read (wl-paste), subprocess management
├── tts_speak.py          Python: Piper TTS synthesis + paplay playback
├── Cargo.toml            Rust dependencies (rdev, anyhow, ctrlc, libc)
├── requirements.txt      Python dependencies (piper-tts, sounddevice, numpy)
└── install.sh            Build + install script (venv, model download, cross-compile support)
```

**Threading model:**
- **Main thread**: state machine (Idle / Speaking) + subprocess management
- **rdev listener thread**: captures raw key events, sends over mpsc channel

**Subprocess isolation:** TTS processes are spawned with `setsid()` in their own process group. Stopping playback sends `SIGKILL` to the negative PID, killing the entire group (shell, Python, paplay) instantly.

**Installed files:**

| Path | Purpose |
|------|---------|
| `~/.local/bin/voice-speak` | Compiled binary |
| `~/.local/bin/tts_speak.py` | Python TTS script |
| `~/.local/bin/tts_speak_wrapper.sh` | Venv-aware wrapper (auto-generated) |
| `~/.local/share/voice-speak/venv/` | Python virtual environment |
| `~/.local/share/voice-speak/models/` | Piper ONNX voice models |

## SteamOS Notes

- Packages installed via pacman (`wl-clipboard`) don't survive SteamOS major updates -- re-run the pacman commands after updates
- User must be in the `input` group for rdev hotkeys: `sudo usermod -aG input $USER` (then reboot)
- Filesystem is read-only by default; `install.sh` documents unlock/re-lock via `steamos-readonly`
- Everything in `~/.local/` (binary, venv, models) and `~/.rustup/` survives updates
- The `input` group membership survives updates
- The only fragile component is `wl-clipboard` installed via pacman

## Known Limitations

- Electron apps (Discord, VS Code) do not populate the Wayland PRIMARY selection -- use Ctrl+C first, then press the hotkey
- Game Mode (Gamescope) may not forward key events to rdev -- use Desktop Mode
- No config file -- all settings require editing `src/main.rs` and rebuilding
- The binary may need to be cross-compiled if SteamOS lacks ALSA development headers
- Audio output uses the system default PulseAudio/PipeWire sink
- Piper pauses are controlled internally by punctuation -- there is no dedicated pause parameter

## License

MIT -- see [LICENSE](../../LICENSE)
