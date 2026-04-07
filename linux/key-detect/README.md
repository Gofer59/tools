# key-detect

Minimal key code detection utility. Press any key and see its `rdev::Key` name or raw keycode. Useful for finding the correct key names to use in config files for other tools (threshold-filter, deck-reader, screen-ocr, etc.).

## Platform

Linux (X11). Requires the `input` group for global key detection.

## Prerequisites

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# User must be in the input group (for rdev)
sudo usermod -aG input $USER
# Then reboot
```

## Installation

```bash
cd key-detect
cargo build --release
cp target/release/key-detect ~/.local/bin/
```

## Usage

```bash
key-detect
```

Press any key. The output shows:

```
Key detection utility — press any key (Ctrl+C to quit)
Look for Key::Unknown(N) values to use in your rdev Config.

KeyPress    F9
KeyRelease  F9
KeyPress    Key::Unknown(191)   <- use Key::Unknown(191) in Config
KeyRelease  Key::Unknown(191)   <- use Key::Unknown(191) in Config
```

- Named keys (F1-F12, Escape, etc.) show their `rdev::Key` variant name
- Unnamed keys show `Key::Unknown(N)` — use that exact code in your tool config
- Press **Ctrl+C** to quit

## Architecture

```
src/
  main.rs    Single-file utility: rdev listener that prints key events
```

## Known Limitations

- Requires the `input` group on Linux (rdev accesses `/dev/input/*`)
- Only detects keyboard events, not mouse or gamepad

## License

MIT — see [LICENSE](../../LICENSE)
