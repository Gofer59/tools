# SteamDeck Tools

Utilities designed for SteamOS 3.x (Wayland). These are Wayland-compatible variants of the Linux tools, using `slurp`/`grim` instead of `slop`/`maim`, and `ydotool` instead of `xdotool`.

## Tools

| Tool | Description |
|------|-------------|
| [deck-reader](deck-reader/) | Multi-hotkey OCR + TTS unified tool |
| [screen-ocr](screen-ocr/) | Hotkey-triggered screen region OCR + TTS |
| [voice-prompt](voice-prompt/) | Push-to-talk speech-to-text (Whisper, offline) |
| [voice-speak](voice-speak/) | Text-to-speech for highlighted text (Piper, offline) |
| [threshold-filter](threshold-filter/) | Live screen binary threshold filter |

## SteamOS Notes

- **Read-only filesystem:** Install scripts handle `steamos-readonly disable/enable` automatically
- **Packages don't survive OS updates:** Re-run `install.sh` after each SteamOS update
- **Input group required:** Global hotkeys need the user in the `input` group:
  ```bash
  sudo usermod -aG input $USER
  # Then reboot
  ```
- **Build on dev machine:** If cargo is not available on the Deck, cross-compile on a Linux machine and copy the binary over
