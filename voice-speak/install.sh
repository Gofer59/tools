#!/usr/bin/env bash
set -euo pipefail

TOOL="voice-speak"
DATA="$HOME/.local/share/$TOOL"
BIN="$HOME/.local/bin"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# ── 1. Detect package manager and install system dependencies ────────────────

if command -v apt-get &>/dev/null; then
    echo "[install] apt: installing system dependencies…"
    sudo apt-get update -qq
    sudo apt-get install -y \
        libwebkit2gtk-4.1-dev \
        libssl-dev \
        libayatana-appindicator3-dev \
        libxdo-dev \
        python3 \
        python3-venv \
        xclip \
        wl-clipboard
elif command -v dnf &>/dev/null; then
    echo "[install] dnf: installing system dependencies…"
    sudo dnf install -y \
        webkit2gtk4.1-devel \
        openssl-devel \
        libayatana-appindicator-devel \
        libxdo-devel \
        python3 \
        xclip \
        wl-clipboard
elif command -v pacman &>/dev/null; then
    echo "[install] pacman: installing system dependencies…"
    sudo pacman -S --needed --noconfirm \
        webkit2gtk-4.1 \
        openssl \
        python \
        xdotool \
        wl-clipboard
else
    echo "[warn] No supported package manager found (apt/dnf/pacman)."
    echo "       Please install the following manually:"
    echo "         libwebkit2gtk-4.1-dev, libssl-dev, libayatana-appindicator3-dev,"
    echo "         libxdo-dev, python3, python3-venv, xclip, wl-clipboard"
fi

# ── 2. Create data directory and Python venv ────────────────────────────────

echo "[install] Setting up Python venv at $DATA/venv…"
mkdir -p "$DATA"
python3 -m venv "$DATA/venv"
"$DATA/venv/bin/pip" install --quiet --upgrade pip
"$DATA/venv/bin/pip" install --quiet piper-tts numpy

# ── 3. Install binary ────────────────────────────────────────────────────────

mkdir -p "$BIN"
install -m 0755 "$SCRIPT_DIR/src-tauri/target/release/$TOOL" "$BIN/$TOOL"
echo "[install] Binary installed to $BIN/$TOOL"

# ── 4. Install daemon script ─────────────────────────────────────────────────

install -m 0644 "$SCRIPT_DIR/python/piper_daemon.py" "$DATA/piper_daemon.py"
echo "[install] Daemon script installed to $DATA/piper_daemon.py"

# ── 5. Create .desktop entry ─────────────────────────────────────────────────

APPS="$HOME/.local/share/applications"
mkdir -p "$APPS"
cat > "$APPS/$TOOL.desktop" <<EOF
[Desktop Entry]
Name=Voice Speak
Comment=TTS for highlighted text
Exec=$BIN/$TOOL
Icon=$DATA/icon.png
Terminal=false
Type=Application
Categories=Utility;Accessibility;
StartupNotify=false
EOF
echo "[install] Desktop entry installed to $APPS/$TOOL.desktop"

echo ""
echo "Installed. Run 'voice-speak' or launch from your app menu."
