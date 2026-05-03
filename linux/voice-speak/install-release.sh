#!/usr/bin/env bash
set -euo pipefail
TOOL="voice-speak"
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  BIN="./${TOOL}-x86_64" ;;
  aarch64) BIN="./${TOOL}-aarch64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac
[ -f "$BIN" ] || { echo "Binary not found: $BIN" >&2; exit 1; }
echo "Installing $TOOL for $ARCH..."

# 1. Binary
mkdir -p ~/.local/bin
install -m 755 "$BIN" ~/.local/bin/$TOOL
echo "  Binary → ~/.local/bin/$TOOL"

# 2. Config (never overwrite existing)
mkdir -p ~/.config/$TOOL
if [ ! -f ~/.config/$TOOL/config.toml ]; then
    cp config.toml ~/.config/$TOOL/config.toml
    echo "  Config → ~/.config/$TOOL/config.toml (new)"
else
    echo "  Config → ~/.config/$TOOL/config.toml (existing, not overwritten)"
fi

# 3. Python script
mkdir -p ~/.local/share/$TOOL
cp python/tts_speak.py ~/.local/share/$TOOL/
echo "  Script → ~/.local/share/$TOOL/tts_speak.py"

# 4. Python venv + piper-tts
VENV="$HOME/.local/share/$TOOL/venv"
if [ ! -d "$VENV" ]; then
    echo "  Creating Python venv..."
    python3 -m venv "$VENV"
fi
echo "  Installing piper-tts..."
"$VENV/bin/pip" install --quiet piper-tts sounddevice numpy
echo "  Python deps installed."

# 5. Wrapper script
cat > ~/.local/bin/tts_speak_wrapper.sh <<EOF
#!/usr/bin/env bash
exec "$VENV/bin/python3" "$HOME/.local/share/$TOOL/tts_speak.py" "\$@"
EOF
chmod +x ~/.local/bin/tts_speak_wrapper.sh
echo "  Wrapper → ~/.local/bin/tts_speak_wrapper.sh"

# 6. Download Piper models
MODEL_DIR="$HOME/.local/share/$TOOL/models"
mkdir -p "$MODEL_DIR"
PIPER_BASE="https://huggingface.co/rhasspy/piper-voices/resolve/main"

download_piper_model() {
    local name="$1"
    local url_path="$2"
    if [ ! -f "$MODEL_DIR/${name}.onnx" ]; then
        echo "  Downloading Piper model: $name (~61 MB)..."
        curl -L --fail --progress-bar \
            -o "$MODEL_DIR/${name}.onnx" \
            "$PIPER_BASE/${url_path}/${name}.onnx"
        curl -L --fail -s \
            -o "$MODEL_DIR/${name}.onnx.json" \
            "$PIPER_BASE/${url_path}/${name}.onnx.json"
        echo "  $name installed."
    else
        echo "  $name already present, skipping."
    fi
}

download_piper_model "en_US-lessac-medium" "en/en_US/lessac/medium"
download_piper_model "fr_FR-siwis-medium"  "fr/fr_FR/siwis/medium"

# 7. Runtime dep check
MISSING=""
for dep in paplay; do
    command -v "$dep" >/dev/null 2>&1 || MISSING="$MISSING $dep"
done
[ -n "$MISSING" ] && echo "  Missing runtime deps:$MISSING — install: sudo apt install$MISSING"

# 8. Desktop entries
mkdir -p ~/.config/autostart
cat > ~/.config/autostart/${TOOL}.desktop <<DESKTOP
[Desktop Entry]
Type=Application
Name=voice-speak
Exec=$HOME/.local/bin/$TOOL --daemon
Hidden=false
X-GNOME-Autostart-enabled=true
Comment=Text-to-speech daemon
DESKTOP

mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/${TOOL}.desktop <<DESKTOP
[Desktop Entry]
Type=Application
Name=voice-speak Settings
Exec=$HOME/.local/bin/$TOOL
Icon=audio-output-speaker
Comment=Configure voice-speak settings
Categories=Utility;Accessibility;
DESKTOP

echo ""
echo "Done! $TOOL installed."
echo "  Run daemon:    $TOOL --daemon"
echo "  Open settings: $TOOL"
