# Voice Prompt (SteamDeck)

Push-to-talk speech-to-text that types your words at the cursor in any application on Linux. Hold a key combo, speak naturally, release the key, and the transcript appears at your cursor within seconds. Uses faster-whisper (CTranslate2) for fully offline speech recognition and xdotool for text injection. Supports English and French.

## Platform

SteamDeck (SteamOS 3.x / KDE Plasma / Wayland)

Also works on standard Linux desktops (Debian, Ubuntu, Arch) under X11 or Wayland (via XWayland).

## Prerequisites

- **Rust 1.70+** and **Cargo** (for building)
- **Python 3** (pre-installed on SteamOS and most Linux distributions)
- **xdotool** (text injection via XWayland)
- **libasound2-dev** / **alsa-lib** (build dependency for cpal audio capture)
- **faster-whisper** (Python speech recognition library)

> **SteamOS read-only filesystem:** On SteamOS, installing system packages requires temporarily unlocking the filesystem:
> ```bash
> sudo steamos-readonly disable
> sudo pacman -S --noconfirm xdotool alsa-lib python
> sudo steamos-readonly enable
> ```

> **Wayland note:** `xdotool` works under XWayland (most apps on KDE Plasma run under XWayland by default). For pure Wayland windows, replace `xdotool` with `ydotool` in `inject_text()` in `src/main.rs` and install `ydotool`.

## Installation

```bash
cd voice-prompt
chmod +x install.sh
./install.sh
```

The installer will:
1. Check for system dependencies (cargo, python3, xdotool)
2. Ensure ALSA development headers are installed
3. Install faster-whisper Python package
4. Build the Rust binary in release mode
5. Install `voice-prompt` and `whisper_transcribe.py` to `~/.local/bin/`

Ensure `~/.local/bin` is in your PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## Usage

```bash
voice-prompt              # English speech recognition (default)
voice-prompt -l fr        # French speech recognition
voice-prompt --language fr
```

| Action | Key combo |
|--------|-----------|
| **Start recording** | Hold **Left Meta** (Super), press **S** |
| **Stop + transcribe** | Release **S** (Meta may stay held) |

1. Focus any text input (terminal, browser, text editor, etc.)
2. Hold **Left Meta**, press **S** -- recording starts
3. Speak naturally
4. Release **S** -- recording stops, transcription runs
5. Text appears at your cursor in ~1--3 seconds

## Configuration

All configuration is done by editing `src/main.rs` in the `Config` struct, then rebuilding with `./install.sh`.

### Push-to-talk chord

```rust
// Default: Left Meta + S
modifier_key: Some(Key::MetaLeft),  trigger_key: Key::KeyS,

// Alternative: single key F9 (no modifier)
modifier_key: None,                 trigger_key: Key::F9,

// Alternative: Left Alt + S
modifier_key: Some(Key::Alt),       trigger_key: Key::KeyS,
```

### Whisper model

Edit `Config::whisper_model`:

| Model | Size | Speed (CPU) | Accuracy |
|-------|------|-------------|----------|
| `"tiny"` | 75 MB | very fast | low |
| `"base"` | 145 MB | fast | good |
| `"small"` | 488 MB | medium | better (default) |
| `"medium"` | 1.5 GB | slow | great |
| `"large-v3"` | 3 GB | very slow | best |

### Language

Use the `-l` / `--language` CLI flag:

```bash
voice-prompt -l en        # English (default)
voice-prompt -l fr        # French
```

### Autostart on login

Create `~/.config/autostart/voice-prompt.desktop`:

```ini
[Desktop Entry]
Type=Application
Name=voice-prompt
Exec=voice-prompt
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
```

## Architecture

```
voice-prompt/
├── src/main.rs                  Rust binary: push-to-talk state machine, audio capture, subprocess dispatch
├── python/whisper_transcribe.py Python transcription script (faster-whisper, CTranslate2, int8, CPU)
├── Cargo.toml                   Rust dependencies (cpal, hound, rdev, tempfile, anyhow, ctrlc)
└── install.sh                   Build + install script
```

**Threading model:**
- **Main thread**: state machine (Idle / Recording) + calls Python subprocess
- **rdev listener thread**: captures raw key events, sends over mpsc channel
- **cpal stream thread**: pushes audio samples (f32 PCM) over another channel

**Data flow:**
1. cpal captures microphone input as f32 PCM samples
2. Samples are converted to i16 and written to a temporary WAV file (hound)
3. Python script runs faster-whisper on the WAV file, prints transcript to stdout
4. xdotool types the transcript at the current cursor position

**Installed files:**

| Path | Purpose |
|------|---------|
| `~/.local/bin/voice-prompt` | Compiled binary |
| `~/.local/bin/whisper_transcribe.py` | Python transcription script |

## SteamOS Notes

- Packages installed via pacman (`xdotool`, `alsa-lib`) don't survive SteamOS major updates -- re-run `install.sh` after updates
- User must be in the `input` group for rdev hotkeys: `sudo usermod -aG input $USER` (then reboot)
- Filesystem is read-only by default; unlock with `sudo steamos-readonly disable` before installing system packages
- The Rust toolchain (`~/.rustup/`, `~/.cargo/`) and installed files (`~/.local/bin/`) survive updates
- `xdotool` works under XWayland on KDE Plasma; for pure Wayland windows, use `ydotool` instead

## Known Limitations

- `xdotool` only works under XWayland -- pure Wayland windows require `ydotool` (code change needed)
- The ~1--3 second transcription delay means you should not click away from the target window before text appears
- Whisper model download happens on first run of the Python script and can be slow (~500 MB for "small")
- No config file -- all settings require editing `src/main.rs` and rebuilding
- Audio capture uses the system default input device (configured in PulseAudio/PipeWire mixer)
- Game Mode (Gamescope) may not forward key events to rdev -- use Desktop Mode

## License

MIT -- see [LICENSE](../../LICENSE)
