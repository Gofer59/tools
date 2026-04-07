#!/usr/bin/env bash
# install.sh — Build voice-speak and install everything
#
# Run once:
#   chmod +x install.sh && ./install.sh
#
# After that just run:  voice-speak
# (assuming ~/.local/bin is in your PATH — it is by default on Linux Mint)

set -euo pipefail

INSTALL_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.local/share/voice-speak"
VENV_DIR="$DATA_DIR/venv"
MODEL_DIR="$DATA_DIR/models"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Piper voice models (one per supported language)
EN_VOICE_NAME="en_US-lessac-medium"
EN_VOICE_URL="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/lessac/medium/en_US-lessac-medium.onnx"
EN_VOICE_JSON_URL="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json"

FR_VOICE_NAME="fr_FR-siwis-medium"
FR_VOICE_URL="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/fr/fr_FR/siwis/medium/fr_FR-siwis-medium.onnx"
FR_VOICE_JSON_URL="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/fr/fr_FR/siwis/medium/fr_FR-siwis-medium.onnx.json"

echo "════════════════════════════════════════"
echo " voice-speak installer"
echo "════════════════════════════════════════"

# ── 1. System dependencies ───────────────────────────────────────────────────
echo ""
echo "▶ Checking system dependencies…"

MISSING=()
command -v cargo   >/dev/null 2>&1 || MISSING+=("cargo (rustup)")
command -v python3 >/dev/null 2>&1 || MISSING+=("python3")

# Check for clipboard tools
if [ -n "${WAYLAND_DISPLAY:-}" ]; then
    command -v wl-paste >/dev/null 2>&1 || MISSING+=("wl-clipboard")
else
    command -v xclip >/dev/null 2>&1 || MISSING+=("xclip")
fi

if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo "  Missing: ${MISSING[*]}"
    echo "  Install with:"
    echo "    sudo apt install xclip          # for X11"
    echo "    sudo apt install wl-clipboard   # for Wayland"
    echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "  ✓ cargo, python3, clipboard tool found"

# ── 2. ALSA development headers (needed by rdev) ────────────────────────────
echo ""
echo "▶ Ensuring ALSA dev headers are installed…"
if ! dpkg -s libasound2-dev >/dev/null 2>&1; then
    echo "  Installing libasound2-dev…"
    sudo apt-get install -y libasound2-dev
else
    echo "  ✓ libasound2-dev already installed"
fi

# ── 3. Python virtual environment + dependencies ────────────────────────────
echo ""
echo "▶ Setting up Python virtual environment…"
mkdir -p "$DATA_DIR"

if [ ! -d "$VENV_DIR" ]; then
    python3 -m venv "$VENV_DIR"
    echo "  ✓ Created venv at $VENV_DIR"
else
    echo "  ✓ Venv already exists at $VENV_DIR"
fi

"$VENV_DIR/bin/pip" install --quiet --upgrade pip
"$VENV_DIR/bin/pip" install --quiet -r "$SCRIPT_DIR/requirements.txt"
echo "  ✓ Python dependencies installed (piper-tts, sounddevice, numpy)"

# ── 4. Download Piper voice models ──────────────────────────────────────────
echo ""
echo "▶ Downloading Piper voice models…"
mkdir -p "$MODEL_DIR"

# English model
EN_ONNX="$MODEL_DIR/$EN_VOICE_NAME.onnx"
EN_JSON="$MODEL_DIR/$EN_VOICE_NAME.onnx.json"

if [ -f "$EN_ONNX" ] && [ -f "$EN_JSON" ]; then
    echo "  ✓ English model already downloaded ($EN_VOICE_NAME)"
else
    echo "  Downloading $EN_VOICE_NAME.onnx…"
    curl -L --progress-bar -o "$EN_ONNX" "$EN_VOICE_URL"
    echo "  Downloading $EN_VOICE_NAME.onnx.json…"
    curl -L --progress-bar -o "$EN_JSON" "$EN_VOICE_JSON_URL"
    echo "  ✓ English model downloaded"
fi

# French model
FR_ONNX="$MODEL_DIR/$FR_VOICE_NAME.onnx"
FR_JSON="$MODEL_DIR/$FR_VOICE_NAME.onnx.json"

if [ -f "$FR_ONNX" ] && [ -f "$FR_JSON" ]; then
    echo "  ✓ French model already downloaded ($FR_VOICE_NAME)"
else
    echo "  Downloading $FR_VOICE_NAME.onnx…"
    curl -L --progress-bar -o "$FR_ONNX" "$FR_VOICE_URL"
    echo "  Downloading $FR_VOICE_NAME.onnx.json…"
    curl -L --progress-bar -o "$FR_JSON" "$FR_VOICE_JSON_URL"
    echo "  ✓ French model downloaded"
fi

# ── 5. Build Rust binary ────────────────────────────────────────────────────
echo ""
echo "▶ Building Rust binary (release mode)…"
cd "$SCRIPT_DIR"
cargo build --release
echo "  ✓ Build complete"

# ── 6. Install to ~/.local/bin ───────────────────────────────────────────────
echo ""
echo "▶ Installing to $INSTALL_DIR…"
mkdir -p "$INSTALL_DIR"

cp target/release/voice-speak "$INSTALL_DIR/voice-speak"
cp tts_speak.py "$INSTALL_DIR/tts_speak.py"
chmod +x "$INSTALL_DIR/voice-speak"
chmod +x "$INSTALL_DIR/tts_speak.py"

# Create a wrapper that activates the venv before running the Python script.
# The Rust binary calls python3, so we point it to the venv's python3.
cat > "$INSTALL_DIR/tts_speak_wrapper.sh" << WRAPPER
#!/usr/bin/env bash
# Auto-generated wrapper that runs tts_speak.py inside the voice-speak venv.
exec "$VENV_DIR/bin/python3" "$INSTALL_DIR/tts_speak.py" "\$@"
WRAPPER
chmod +x "$INSTALL_DIR/tts_speak_wrapper.sh"

echo "  ✓ voice-speak       → $INSTALL_DIR/voice-speak"
echo "  ✓ tts_speak.py      → $INSTALL_DIR/tts_speak.py"
echo "  ✓ wrapper script    → $INSTALL_DIR/tts_speak_wrapper.sh"

# ── 7. PATH check ───────────────────────────────────────────────────────────
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

# ── 8. Done ──────────────────────────────────────────────────────────────────
echo ""
echo "════════════════════════════════════════"
echo " Installation complete!"
echo "════════════════════════════════════════"
echo ""
echo "Run manually:       voice-speak"
echo "                    voice-speak -l fr    (French)"
echo "                    voice-speak -l en    (English, default)"
echo ""
echo "Hotkey:             Right Alt (AltGr)"
echo "  Press once  →  speak highlighted text"
echo "  Press again →  stop playback"
echo ""
echo "To change the hotkey: edit src/main.rs → Config::hotkey"
echo "                      then re-run this installer."
