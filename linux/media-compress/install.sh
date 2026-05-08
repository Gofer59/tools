#!/usr/bin/env bash
# install.sh — Install media-compress scripts to ~/.local/bin
#
# Run once:
#   chmod +x install.sh && ./install.sh
#
# After that just run:  compress-videos <file-or-folder> [crf]
# (assuming ~/.local/bin is in your PATH)

set -euo pipefail

INSTALL_DIR="$HOME/.local/bin"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "════════════════════════════════════════"
echo " media-compress installer"
echo "════════════════════════════════════════"

# ── 0. Detect package manager ───────────────────────────────────────────────
PKG_MANAGER=""
if command -v apt-get >/dev/null 2>&1; then
    PKG_MANAGER="apt"
elif command -v pacman >/dev/null 2>&1; then
    PKG_MANAGER="pacman"
else
    echo "⚠ Could not detect apt or pacman. Install system tools manually."
    PKG_MANAGER="unknown"
fi

# ── 1. System dependencies ──────────────────────────────────────────────────
echo ""
echo "▶ Checking system dependencies…"

REQUIRED=(ffmpeg jpegoptim optipng cwebp avifenc mogrify)
MISSING=()
for cmd in "${REQUIRED[@]}"; do
    command -v "$cmd" >/dev/null 2>&1 || MISSING+=("$cmd")
done

if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo "  Missing: ${MISSING[*]}"
    echo ""
    if [ "$PKG_MANAGER" = "apt" ]; then
        echo "  Install with:"
        echo "    sudo apt install -y ffmpeg jpegoptim optipng webp libavif-bin imagemagick"
    elif [ "$PKG_MANAGER" = "pacman" ]; then
        echo "  Install with:"
        echo "    sudo pacman -S ffmpeg jpegoptim optipng libwebp libavif imagemagick"
    fi
    echo ""
    echo "  Re-run this installer after installing the system tools."
    exit 1
fi

echo "  ✓ all required tools found"

# ── 2. Install scripts ──────────────────────────────────────────────────────
echo ""
echo "▶ Installing scripts to $INSTALL_DIR…"
mkdir -p "$INSTALL_DIR"

for script in compress-images compress-videos; do
    if [ ! -f "$SCRIPT_DIR/$script" ]; then
        echo "  ✗ $script not found in $SCRIPT_DIR" >&2
        exit 1
    fi
    cp "$SCRIPT_DIR/$script" "$INSTALL_DIR/$script"
    chmod +x "$INSTALL_DIR/$script"
    echo "  ✓ $script → $INSTALL_DIR/$script"
done

# ── 3. PATH check ──────────────────────────────────────────────────────────
echo ""
if echo "$PATH" | grep -q "$HOME/.local/bin"; then
    echo "✓ $HOME/.local/bin is already in your PATH"
else
    echo "⚠ Add this to your ~/.bashrc or ~/.zshrc:"
    echo ""
    echo '    export PATH="$HOME/.local/bin:$PATH"'
    echo ""
    echo "  Then run:  source ~/.bashrc"
fi

# ── 4. Done ─────────────────────────────────────────────────────────────────
echo ""
echo "════════════════════════════════════════"
echo " Installation complete!"
echo "════════════════════════════════════════"
echo ""
echo "Usage:"
echo "  compress-videos <file-or-folder> [crf]      # H.265 video, default CRF 28"
echo "  compress-images <file-or-folder> [quality]  # JPEG/PNG optimize, default quality 80"
