#!/usr/bin/env bash
set -euo pipefail
TOOL="threshold-filter"
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  BIN="./${TOOL}-x86_64" ;;
  aarch64) BIN="./${TOOL}-aarch64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac
[ -f "$BIN" ] || { echo "Binary not found: $BIN" >&2; exit 1; }
mkdir -p ~/.local/bin
install -m 755 "$BIN" ~/.local/bin/$TOOL
mkdir -p ~/.config/$TOOL
if [ ! -f ~/.config/$TOOL/config.toml ]; then
    cp config.toml ~/.config/$TOOL/config.toml
    echo "  Config → ~/.config/$TOOL/config.toml (new)"
else
    echo "  Config → ~/.config/$TOOL/config.toml (existing, not overwritten)"
fi
echo "Done! Run: $TOOL"
