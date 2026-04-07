# Desktop Tools

A collection of open-source desktop utilities for OCR, text-to-speech, screen capture, and media processing. Most tools follow a **Rust + Python** architecture with global hotkey activation.

## Tools

| Tool | Description | Platforms |
|------|-------------|-----------|
| [threshold-filter](linux/threshold-filter/) | Live screen binary threshold filter for OCR preprocessing | Linux, Windows, SteamDeck |
| [voice-prompt](linux/voice-prompt/) | Push-to-talk speech-to-text (Whisper, offline) | Linux, SteamDeck |
| [voice-speak](linux/voice-speak/) | TTS for highlighted text (Piper, offline) | Linux, SteamDeck |
| [screen-ocr](linux/screen-ocr/) | Hotkey-triggered screen region OCR + TTS | Linux, SteamDeck |
| [deck-reader](steamdeck/deck-reader/) | Multi-hotkey OCR + TTS unified tool | SteamDeck |
| [book-digitize](linux/book-digitize/) | Video-to-markdown OCR for lectures/books | Linux |
| [gamebook-digitize](linux/gamebook-digitize/) | Gamebook video to interactive HTML | Linux |
| [book-reader](android/book-reader/) | OCR + TTS book reader app | Android |
| [media-compress](linux/media-compress/) | Video/image/audio compression scripts | Linux |
| [key-detect](linux/key-detect/) | Key code detection utility | Linux |

## Platforms

- **[Linux](linux/)** — X11 desktop tools (8 tools)
- **[Windows](windows/)** — Windows-compatible tools (1 tool)
- **[SteamDeck](steamdeck/)** — SteamOS/Wayland variants (5 tools)
- **[Android](android/)** — Android apps (1 tool)

## Shared Architecture

The hotkey-based tools (voice-prompt, voice-speak, screen-ocr, deck-reader, threshold-filter) share a common pattern:

- **Hotkey listener:** [rdev](https://crates.io/crates/rdev) crate for global key events
- **Process isolation:** `setsid()` + `kill(-pid, SIGKILL)` for subprocess cleanup
- **Audio output:** `paplay` (PulseAudio/PipeWire)
- **TTS engine:** [Piper](https://github.com/rhasspy/piper) ONNX models, fully offline
- **STT engine:** [faster-whisper](https://github.com/SYSTRAN/faster-whisper) (CTranslate2, CPU)
- **OCR engine:** [Tesseract](https://github.com/tesseract-ocr/tesseract) via pytesseract
- **Clipboard:** `xclip` (X11) / `wl-clipboard` (Wayland)
- **Text injection:** `xdotool` (X11) / `ydotool` (Wayland)
- **Config:** TOML config files at `~/.config/<tool>/config.toml`

## License

[MIT](LICENSE)
