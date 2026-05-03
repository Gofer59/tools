#!/usr/bin/env bash
set -euo pipefail
TOOL="key-detect"
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  BIN="./${TOOL}-x86_64" ;;
  aarch64) BIN="./${TOOL}-aarch64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac
[ -f "$BIN" ] || { echo "Binary not found: $BIN" >&2; exit 1; }
mkdir -p ~/.local/bin
install -m 755 "$BIN" ~/.local/bin/$TOOL
echo "Done! Run: $TOOL"
echo "  Prints key codes to stdout. Press Ctrl+C to stop."
