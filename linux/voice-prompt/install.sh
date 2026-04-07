#!/usr/bin/env bash
# install.sh — Build voice-prompt and install everything to ~/.local/bin
#
# Run once:
#   chmod +x install.sh && ./install.sh
#
# After that just run:  voice-prompt
# (assuming ~/.local/bin is in your PATH — it is by default on Linux Mint)

set -euo pipefail

INSTALL_DIR="$HOME/.local/bin"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "════════════════════════════════════════"
echo " voice-prompt installer"
echo "════════════════════════════════════════"

# ── 1. System dependencies ───────────────────────────────────────────────────
echo ""
echo "▶ Checking system dependencies…"

MISSING=()
command -v xdotool >/dev/null 2>&1 || MISSING+=("xdotool")
command -v cargo   >/dev/null 2>&1 || MISSING+=("cargo (rustup)")
command -v python3 >/dev/null 2>&1 || MISSING+=("python3")

if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo "  Missing: ${MISSING[*]}"
    echo "  Install with:"
    echo "    sudo apt install xdotool"
    echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "  ✓ xdotool, cargo, python3 found"

# ── 2. ALSA development headers (needed by cpal) ─────────────────────────────
echo ""
echo "▶ Ensuring ALSA dev headers are installed…"
if ! dpkg -s libasound2-dev >/dev/null 2>&1; then
    echo "  Installing libasound2-dev…"
    sudo apt-get install -y libasound2-dev
else
    echo "  ✓ libasound2-dev already installed"
fi

# ── 3. Python dependency ─────────────────────────────────────────────────────
echo ""
echo "▶ Installing Python dependency (faster-whisper)…"
pip install faster-whisper --break-system-packages --quiet
echo "  ✓ faster-whisper installed"

# ── 4. Build Rust binary ─────────────────────────────────────────────────────
echo ""
echo "▶ Building Rust binary (release mode)…"
cd "$SCRIPT_DIR"
cargo build --release
echo "  ✓ Build complete"

# ── 5. Install to ~/.local/bin ───────────────────────────────────────────────
echo ""
echo "▶ Installing to $INSTALL_DIR…"
mkdir -p "$INSTALL_DIR"

cp target/release/voice-prompt "$INSTALL_DIR/voice-prompt"
cp python/whisper_transcribe.py "$INSTALL_DIR/whisper_transcribe.py"
chmod +x "$INSTALL_DIR/voice-prompt"
chmod +x "$INSTALL_DIR/whisper_transcribe.py"

echo "  ✓ voice-prompt → $INSTALL_DIR/voice-prompt"
echo "  ✓ whisper_transcribe.py → $INSTALL_DIR/whisper_transcribe.py"

# ── 6. PATH check ────────────────────────────────────────────────────────────
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

# ── 7. Autostart hint ────────────────────────────────────────────────────────
echo ""
echo "════════════════════════════════════════"
echo " Installation complete!"
echo "════════════════════════════════════════"
echo ""
echo "Run manually:       voice-prompt"
echo ""
echo "To start on login, add to your desktop autostart:"
echo "  ~/.config/autostart/voice-prompt.desktop"
echo ""
echo "Example .desktop file:"
echo "---"
cat <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=voice-prompt
Exec=voice-prompt
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
DESKTOP
echo "---"
echo ""
echo "Push-to-talk chord: Left Meta + S"
echo "Language flag:      voice-prompt -l fr  (default: en)"
echo "To change the chord: edit src/main.rs → Config::modifier_key / trigger_key"
echo "                     then re-run this installer."
