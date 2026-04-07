# Linux Tools

Desktop utilities for Linux (X11). Requires an X11 session for hotkey-based tools.

## Tools

| Tool | Description |
|------|-------------|
| [threshold-filter](threshold-filter/) | Live screen binary threshold filter for OCR preprocessing |
| [voice-prompt](voice-prompt/) | Push-to-talk speech-to-text (Whisper, offline) |
| [voice-speak](voice-speak/) | Text-to-speech for highlighted text (Piper, offline) |
| [screen-ocr](screen-ocr/) | Hotkey-triggered screen region OCR + TTS |
| [book-digitize](book-digitize/) | Video-to-markdown OCR for lectures and books |
| [gamebook-digitize](gamebook-digitize/) | Gamebook video to Markdown + interactive HTML |
| [media-compress](media-compress/) | Video, image, and audio compression scripts |
| [key-detect](key-detect/) | Key code detection utility for configuring hotkeys |

## Common Dependencies

Most Rust-based tools need:

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build dependencies (Ubuntu/Debian)
sudo apt-get install pkg-config libxcb-render0-dev libxcb-shape0-dev \
    libxcb-xfixes0-dev libxkbcommon-dev libgl-dev

# Runtime dependencies
sudo apt-get install xdotool xclip
```

Python-based tools use virtual environments created by their `install.sh` scripts.
