#!/usr/bin/env bash
set -euo pipefail
TOOL="screen-ocr"
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
cp python/ocr_extract.py ~/.local/share/$TOOL/
echo "  Script → ~/.local/share/$TOOL/ocr_extract.py"

# 4. Python venv + pytesseract
VENV="$HOME/.local/share/$TOOL/venv"
if [ ! -d "$VENV" ]; then
    echo "  Creating Python venv..."
    python3 -m venv "$VENV"
fi
echo "  Installing Python deps..."
"$VENV/bin/pip" install --quiet -r python/requirements.txt
echo "  Python deps installed."

# 5. Download tessdata
TESS_DIR="$HOME/.local/share/$TOOL/models/tessdata"
mkdir -p "$TESS_DIR"
TESS_BASE="https://github.com/tesseract-ocr/tessdata/raw/main"
for lang_size in "eng:4 MB" "fra:1 MB"; do
    lang="${lang_size%%:*}"
    size="${lang_size##*:}"
    if [ ! -f "$TESS_DIR/${lang}.traineddata" ]; then
        echo "  Downloading tessdata: $lang (~$size)..."
        curl -L --fail --progress-bar \
            -o "$TESS_DIR/${lang}.traineddata" \
            "$TESS_BASE/${lang}.traineddata"
    else
        echo "  tessdata/$lang already present, skipping."
    fi
done

# 6. Download Piper TTS model (en only; screen-ocr reads aloud in English)
PIPER_DIR="$HOME/.local/share/voice-speak/models"
mkdir -p "$PIPER_DIR"
PIPER_BASE="https://huggingface.co/rhasspy/piper-voices/resolve/main"
if [ ! -f "$PIPER_DIR/en_US-lessac-medium.onnx" ]; then
    echo "  Downloading Piper TTS model: en_US-lessac-medium (~61 MB)..."
    curl -L --fail --progress-bar \
        -o "$PIPER_DIR/en_US-lessac-medium.onnx" \
        "$PIPER_BASE/en/en_US/lessac/medium/en_US-lessac-medium.onnx"
    curl -L --fail -s \
        -o "$PIPER_DIR/en_US-lessac-medium.onnx.json" \
        "$PIPER_BASE/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json"
else
    echo "  Piper en_US-lessac-medium already present, skipping."
fi

# 7. Wrapper (sets TESSDATA_PREFIX so pytesseract uses downloaded tessdata)
cat > ~/.local/bin/ocr_extract_wrapper.sh <<EOF
#!/usr/bin/env bash
export TESSDATA_PREFIX="$HOME/.local/share/$TOOL/models"
exec "$VENV/bin/python3" "$HOME/.local/share/$TOOL/ocr_extract.py" "\$@"
EOF
chmod +x ~/.local/bin/ocr_extract_wrapper.sh
echo "  Wrapper → ~/.local/bin/ocr_extract_wrapper.sh"

# 8. Runtime dep check
MISSING=""
for dep in tesseract maim slop xclip paplay; do
    command -v "$dep" >/dev/null 2>&1 || MISSING="$MISSING $dep"
done
[ -n "$MISSING" ] && echo "  Missing runtime deps:$MISSING — install: sudo apt install$MISSING"

# 9. Desktop entries
mkdir -p ~/.config/autostart
cat > ~/.config/autostart/${TOOL}.desktop <<DESKTOP
[Desktop Entry]
Type=Application
Name=screen-ocr
Exec=$HOME/.local/bin/$TOOL --daemon
Hidden=false
X-GNOME-Autostart-enabled=true
Comment=Screen OCR daemon
DESKTOP

mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/${TOOL}.desktop <<DESKTOP
[Desktop Entry]
Type=Application
Name=screen-ocr Settings
Exec=$HOME/.local/bin/$TOOL
Icon=scanner
Comment=Configure screen-ocr settings
Categories=Utility;Accessibility;
DESKTOP

echo ""
echo "Done! $TOOL installed."
echo "  Run daemon:    $TOOL --daemon"
echo "  Open settings: $TOOL"
