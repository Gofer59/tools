#!/usr/bin/env bash
set -euo pipefail

TOOL="voice-prompt"
DATA="$HOME/.local/share/$TOOL"
BIN="$HOME/.local/bin"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== voice-prompt installer ==="

# ---------------------------------------------------------------------------
# 1. Detect package manager and install system dependencies
# ---------------------------------------------------------------------------
install_deps_apt() {
    echo "[*] Installing system dependencies via apt..."
    sudo apt-get update -qq
    # libxdo-dev is critical — enigo 0.2 requires it at build time
    sudo apt-get install -y \
        libwebkit2gtk-4.1-dev \
        libssl-dev \
        libayatana-appindicator3-dev \
        libxdo-dev \
        python3 \
        python3-venv \
        xdotool \
        xclip \
        wl-clipboard \
        wtype \
        portaudio19-dev
}

install_deps_dnf() {
    echo "[*] Installing system dependencies via dnf..."
    sudo dnf install -y \
        webkit2gtk4.1-devel \
        openssl-devel \
        libayatana-appindicator-devel \
        libxdo-devel \
        python3 \
        xdotool \
        xclip \
        wl-clipboard \
        wtype \
        portaudio-devel
}

install_deps_pacman() {
    echo "[*] Installing system dependencies via pacman..."
    sudo pacman -Sy --needed --noconfirm \
        webkit2gtk-4.1 \
        openssl \
        python \
        xdotool \
        xclip \
        wl-clipboard \
        wtype \
        portaudio
}

if command -v apt-get &>/dev/null; then
    install_deps_apt
elif command -v dnf &>/dev/null; then
    install_deps_dnf
elif command -v pacman &>/dev/null; then
    install_deps_pacman
else
    echo "[!] WARNING: Could not detect a supported package manager (apt/dnf/pacman)."
    echo "    Please install the following system packages manually before continuing:"
    echo "      - WebKit2GTK 4.1 dev headers"
    echo "      - OpenSSL dev headers"
    echo "      - libayatana-appindicator dev headers"
    echo "      - libxdo dev headers  (required by enigo 0.2)"
    echo "      - python3 + python3-venv"
    echo "      - xclip and/or wl-clipboard"
    echo "      - PortAudio dev headers"
fi

# ---------------------------------------------------------------------------
# 2. Create data directory and Python venv
# ---------------------------------------------------------------------------
echo "[*] Creating data directory: $DATA"
mkdir -p "$DATA"

echo "[*] Setting up Python virtual environment..."
python3 -m venv "$DATA/venv"
"$DATA/venv/bin/pip" install --upgrade pip --quiet
"$DATA/venv/bin/pip" install faster-whisper --quiet
echo "[+] Python venv ready at $DATA/venv"

# ---------------------------------------------------------------------------
# 3. Install binary
# ---------------------------------------------------------------------------
BINARY="$SCRIPT_DIR/src-tauri/target/release/$TOOL"
if [[ ! -f "$BINARY" ]]; then
    echo "[!] ERROR: Binary not found at $BINARY"
    echo "    Build first with: cd $SCRIPT_DIR && cargo tauri build"
    exit 1
fi

mkdir -p "$BIN"
install -m 0755 "$BINARY" "$BIN/$TOOL"
echo "[+] Binary installed to $BIN/$TOOL"

# ---------------------------------------------------------------------------
# 4. Install whisper daemon script
# ---------------------------------------------------------------------------
DAEMON_SRC="$SCRIPT_DIR/python/whisper_daemon.py"
if [[ ! -f "$DAEMON_SRC" ]]; then
    echo "[!] WARNING: whisper_daemon.py not found at $DAEMON_SRC — skipping."
else
    install -m 0644 "$DAEMON_SRC" "$DATA/whisper_daemon.py"
    echo "[+] Daemon script installed to $DATA/whisper_daemon.py"
fi

# ---------------------------------------------------------------------------
# 5. Create .desktop entry
# ---------------------------------------------------------------------------
APPS_DIR="$HOME/.local/share/applications"
mkdir -p "$APPS_DIR"
cat > "$APPS_DIR/$TOOL.desktop" <<EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=Voice Prompt
Comment=Push-to-talk speech-to-text transcription
Exec=$BIN/$TOOL
Icon=$TOOL
Categories=Utility;Accessibility;
StartupNotify=false
EOF
echo "[+] .desktop entry created at $APPS_DIR/$TOOL.desktop"

# ---------------------------------------------------------------------------
# 6. Ensure $BIN is on PATH (best-effort notice)
# ---------------------------------------------------------------------------
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$BIN"; then
    echo ""
    echo "[!] NOTE: $BIN is not in your PATH."
    echo "    Add the following line to your ~/.bashrc or ~/.zshrc:"
    echo "      export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# ---------------------------------------------------------------------------
# 7. Add user to `input` group (Linux only) — required for live preview.
# ---------------------------------------------------------------------------
# voice-prompt prefers `rdev` (kernel evdev) for hotkey detection because it
# does NOT see synthetic xdotool events. Without it (XGrabKey via tauri-plugin)
# any xdotool injection during PTT hold causes a fork-bomb crash. evdev needs
# read access to /dev/input/event* which is gated by the `input` group on Linux.
if [[ "$(uname -s)" == "Linux" && -n "${USER:-}" ]]; then
    if ! id -nG "$USER" 2>/dev/null | tr ' ' '\n' | grep -qx "input"; then
        echo ""
        echo "[*] Adding $USER to the 'input' group (needed for live in-target-window preview)..."
        if sudo usermod -aG input "$USER"; then
            echo "[+] Done. You MUST log out and back in (or reboot) for this to take effect."
            echo "    Until then voice-prompt falls back to final-only injection."
        else
            echo "[!] Could not add to 'input' group. Run manually:"
            echo "      sudo usermod -aG input \$USER"
            echo "    Then log out and back in. Without this, live preview stays disabled."
        fi
    else
        echo "[+] $USER already in 'input' group — live preview will be enabled."
    fi
fi

echo ""
echo "Installed. Run 'voice-prompt' or launch from your app menu."
