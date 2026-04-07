#!/usr/bin/env bash
# install.sh — Build key-detect on dev machine OR deploy pre-built binary on Steam Deck
#
# Dev machine (Linux Mint):
#   ./install.sh              # builds + installs to ~/.local/bin
#
# Steam Deck:
#   ./install.sh --deploy     # skips cargo build, copies pre-built binary
#
# Then run:  key-detect

set -euo pipefail

INSTALL_DIR="$HOME/.local/bin"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="$SCRIPT_DIR/target/release/key-detect"

echo "════════════════════════════════════════"
echo " key-detect installer"
echo "════════════════════════════════════════"

# ── Build (skip on --deploy) ─────────────────────────────────────────────────
if [[ "${1:-}" == "--deploy" ]]; then
    echo ""
    echo "▶ Deploy mode — skipping cargo build"
    if [[ ! -f "$BINARY" ]]; then
        echo "  ✗ Pre-built binary not found at: $BINARY"
        echo "  Build on your dev machine first, then transfer the whole directory."
        exit 1
    fi
else
    echo ""
    echo "▶ Checking dependencies…"
    if ! command -v cargo >/dev/null 2>&1; then
        echo "  Missing: cargo (rustup)"
        echo "  Install with:"
        echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
    echo "  ✓ cargo found"

    echo ""
    echo "▶ Building Rust binary (release mode)…"
    cd "$SCRIPT_DIR"
    cargo build --release
    echo "  ✓ Build complete"
fi

# ── Install to ~/.local/bin ──────────────────────────────────────────────────
echo ""
echo "▶ Installing to $INSTALL_DIR…"
mkdir -p "$INSTALL_DIR"
cp "$BINARY" "$INSTALL_DIR/key-detect"
chmod +x "$INSTALL_DIR/key-detect"
echo "  ✓ key-detect → $INSTALL_DIR/key-detect"

# ── Done ─────────────────────────────────────────────────────────────────────
echo ""
echo "════════════════════════════════════════"
echo " Installation complete!"
echo "════════════════════════════════════════"
echo ""
echo "Run:  key-detect"
echo "Press keys to see their rdev names/codes, Ctrl+C to quit."
