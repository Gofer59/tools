#!/usr/bin/env bash
set -euo pipefail
TOOL="voice-prompt"
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
cp python/whisper_transcribe.py ~/.local/share/$TOOL/
echo "  Script → ~/.local/share/$TOOL/whisper_transcribe.py"

# 4. Python venv + faster-whisper
VENV="$HOME/.local/share/$TOOL/venv"
if [ ! -d "$VENV" ]; then
    echo "  Creating Python venv..."
    python3 -m venv "$VENV"
fi
echo "  Installing faster-whisper..."
"$VENV/bin/pip" install --quiet faster-whisper
echo "  Python deps installed."

# 5. Wrapper script (binary expects this at ~/.local/bin/whisper_transcribe_wrapper.sh)
cat > ~/.local/bin/whisper_transcribe_wrapper.sh <<EOF
#!/usr/bin/env bash
exec "$VENV/bin/python3" "$HOME/.local/share/$TOOL/whisper_transcribe.py" "\$@"
EOF
chmod +x ~/.local/bin/whisper_transcribe_wrapper.sh
echo "  Wrapper → ~/.local/bin/whisper_transcribe_wrapper.sh"

# 6. Runtime dep check
MISSING=""
for dep in xdotool paplay; do
    command -v "$dep" >/dev/null 2>&1 || MISSING="$MISSING $dep"
done
[ -n "$MISSING" ] && echo "  Missing runtime deps:$MISSING — install: sudo apt install$MISSING"

# 7. Desktop entries
mkdir -p ~/.config/autostart
cat > ~/.config/autostart/${TOOL}.desktop <<DESKTOP
[Desktop Entry]
Type=Application
Name=voice-prompt
Exec=$HOME/.local/bin/$TOOL --daemon
Hidden=false
X-GNOME-Autostart-enabled=true
Comment=Push-to-talk speech transcription daemon
DESKTOP

mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/${TOOL}.desktop <<DESKTOP
[Desktop Entry]
Type=Application
Name=voice-prompt Settings
Exec=$HOME/.local/bin/$TOOL
Icon=audio-input-microphone
Comment=Configure voice-prompt settings
Categories=Utility;Accessibility;
DESKTOP

echo ""
echo "Done! $TOOL installed."
echo "  Run daemon:    $TOOL --daemon"
echo "  Open settings: $TOOL"
echo "  Note: Whisper 'small' model (~244 MB) downloads on first use."
