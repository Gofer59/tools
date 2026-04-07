#!/usr/bin/env bash
set -euo pipefail

INSTALL_DIR="$HOME/.local/bin"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "════════════════════════════════════════"
echo " threshold-filter installer"
echo "════════════════════════════════════════"

# Check cargo
if ! command -v cargo >/dev/null 2>&1; then
    echo "Missing: cargo (Rust toolchain)"
    echo "Install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Linux build dependencies for eframe (egui)
if [ "$(uname)" = "Linux" ]; then
    echo ""
    echo "▶ Checking build dependencies for eframe..."

    MISSING=()
    for lib in xcb xkbcommon; do
        if ! pkg-config --exists "$lib" 2>/dev/null; then
            MISSING+=("$lib")
        fi
    done

    if [ ${#MISSING[@]} -gt 0 ]; then
        echo "Missing libraries: ${MISSING[*]}"
        echo ""

        if command -v apt-get >/dev/null 2>&1; then
            echo "Install with:"
            echo "  sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libgl-dev pkg-config"
        elif command -v pacman >/dev/null 2>&1; then
            echo "Install with:"
            echo "  sudo pacman -S libxcb libxkbcommon mesa pkg-config"
        elif command -v dnf >/dev/null 2>&1; then
            echo "Install with:"
            echo "  sudo dnf install libxcb-devel libxkbcommon-devel mesa-libGL-devel pkg-config"
        fi

        echo ""
        read -p "Continue anyway? [y/N] " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    else
        echo "  All build dependencies found."
    fi

    # Check for region selection tool
    echo ""
    echo "▶ Checking runtime dependencies..."
    if command -v slop >/dev/null 2>&1; then
        echo "  slop found (region selection)"
    else
        echo "  WARNING: slop not found. Region selection will not work."
        echo "  Install with:"
        if command -v apt-get >/dev/null 2>&1; then
            echo "    sudo apt-get install slop"
        elif command -v pacman >/dev/null 2>&1; then
            echo "    sudo pacman -S slop"
        fi
    fi

    if command -v xdotool >/dev/null 2>&1; then
        echo "  xdotool found (window positioning)"
    else
        echo "  WARNING: xdotool not found. Window move buttons will not work."
        echo "  Install with:"
        if command -v apt-get >/dev/null 2>&1; then
            echo "    sudo apt-get install xdotool"
        elif command -v pacman >/dev/null 2>&1; then
            echo "    sudo pacman -S xdotool"
        fi
    fi
fi

echo ""
echo "▶ Building (release mode)..."
cd "$SCRIPT_DIR"
cargo build --release

echo ""
echo "▶ Installing to $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"
cp target/release/threshold-filter "$INSTALL_DIR/threshold-filter"
chmod +x "$INSTALL_DIR/threshold-filter"

echo ""
echo "════════════════════════════════════════"
echo " Installation complete!"
echo "════════════════════════════════════════"
echo ""
echo "Run: threshold-filter"
echo ""
echo "Controls:"
echo "  - On launch, draw a rectangle to select the capture region"
echo "  - Drag slider to adjust black/white threshold (0-255)"
echo "  - Press F10 to select a new region"
echo "  - Toggle 'Always on Top' checkbox"
