# voice-prompt

Push-to-talk speech-to-text that types your words **anywhere** on Linux —
terminals, browsers, text editors, Claude Code CLI, anything.

```
Hold Left-Meta + S → speak → release S → text appears at your cursor
```

## Architecture

```
Rust binary (voice-prompt)
  ├─ rdev       — global key listener (hold / release detection)
  ├─ cpal       — microphone capture → f32 PCM samples
  ├─ hound      — write samples to a temp .wav file
  └─ subprocess → python3 whisper_transcribe.py <wav> <model> <lang>
                     └─ faster-whisper (CTranslate2, int8, CPU)
                            └─ prints transcript to stdout
                  xdotool type -- "<transcript>"
                     └─ injects keystrokes at cursor in any X11 app
```

## Requirements

| Tool | Install |
|------|---------|
| Rust + Cargo | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| libasound2-dev | `sudo apt install libasound2-dev` |
| xdotool | `sudo apt install xdotool` |
| python3 | pre-installed on Linux Mint |
| faster-whisper | `pip install faster-whisper --break-system-packages` |

## Install

```bash
chmod +x install.sh
./install.sh
```

This builds the Rust binary in release mode and copies both
`voice-prompt` and `whisper_transcribe.py` to `~/.local/bin/`.

## Usage

```bash
voice-prompt              # starts listening in English (default); Ctrl-C to quit
voice-prompt -l fr        # French speech recognition
voice-prompt --language fr
```

1. Focus any text input (terminal, browser, text editor…)
2. Hold **Left Meta** (the left Windows/Super key), press **S**
3. Speak naturally
4. Release **S**
5. Text is typed at your cursor in ~1–3 seconds

## Customisation

### Change the push-to-talk chord

Edit `src/main.rs`, `Config::modifier_key` and `Config::trigger_key`:

```rust
// Chord: Left Alt + S
modifier_key: Some(Key::AltLeft),  trigger_key: Key::KeyS,

// Single key: F9 (no modifier)
modifier_key: None,            trigger_key: Key::F9,

// Chord: Right Ctrl + F
modifier_key: Some(Key::ControlRight), trigger_key: Key::KeyF,
```

Available key names: <https://docs.rs/rdev/latest/rdev/enum.Key.html>

Then re-run `./install.sh`.

### Change Whisper model

Edit `src/main.rs`, `Config::whisper_model`:

| Model | Size on disk | Speed (CPU) | Accuracy |
|-------|-------------|-------------|----------|
| `"tiny"` | 75 MB | very fast | low |
| `"base"` | 145 MB | fast | good |
| `"small"` | 488 MB | medium | better ✓ (default) |
| `"medium"` | 1.5 GB | slow | great |
| `"large-v3"` | 3 GB | very slow | best |

### Select speech recognition language

Use the `-l` / `--language` flag:

```bash
voice-prompt -l fr        # French
voice-prompt -l en        # English (default)
voice-prompt --language fr
```

Valid values: `en` (English), `fr` (French).

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

## Troubleshooting

**`No audio input device found`**
→ Check your microphone is connected and selected in the PulseAudio mixer.
→ Try: `arecord -l` to list devices.

**`xdotool: command not found`**
→ `sudo apt install xdotool`

**`faster-whisper not installed`**
→ `pip install faster-whisper --break-system-packages`

**Text injected in wrong window**
→ Make sure the target window is focused *before* you hold the key.
→ There is a ~1–3 s delay after release while Python runs; don't click away.

**`rdev` requires root / doesn't work**
→ On some setups rdev needs access to `/dev/input`. Try:
→ `sudo voice-prompt`  (temporary workaround)
→ Or add yourself to the `input` group:
   `sudo usermod -aG input $USER` then log out and back in.

## Wayland note

`xdotool` works only under **XWayland** (most apps on a Wayland session run
under XWayland by default on Linux Mint 22+).  Pure Wayland windows need
`ydotool` instead — replace `xdotool` with `ydotool` in `inject_text()` in
`src/main.rs` and install it with `sudo apt install ydotool`.
