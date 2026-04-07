#!/usr/bin/env bash
# install.sh — Build voice-speak and install everything
#
# Run once:
#   chmod +x install.sh && ./install.sh
#
# After that just run:  voice-speak
# (assuming ~/.local/bin is in your PATH)
#
# Steam Deck / SteamOS notes
# ──────────────────────────
# SteamOS has an immutable read-only rootfs, so system packages cannot be
# installed with pacman.  Everything here installs into ~/.local/ only.
#
# Build dependency: rdev links against libasound (ALSA) at compile time.
# On SteamOS the runtime .so exists, but the dev headers may be absent.
# If `cargo build` fails with a missing alsa header, two options:
#   A) Build on a normal Linux machine (same x86_64 arch) and scp the binary:
#        scp target/release/voice-speak deck@steamdeck:~/.local/bin/
#      Then re-run install.sh on the Deck — it will skip the build step if the
#      binary is already present in target/release/.
#   B) Use distrobox on the Deck (ships with SteamOS):
#        distrobox enter archlinux -- bash install.sh

set -euo pipefail

INSTALL_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.local/share/voice-speak"
VENV_DIR="$DATA_DIR/venv"
MODEL_DIR="$DATA_DIR/models"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Default Piper voice model
VOICE_NAME="en_US-lessac-medium"
VOICE_URL="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/lessac/medium/en_US-lessac-medium.onnx"
VOICE_JSON_URL="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json"

echo "════════════════════════════════════════"
echo " voice-speak installer"
echo "════════════════════════════════════════"

# ── Detect platform ──────────────────────────────────────────────────────────
IS_STEAMOS=false
if grep -qE '^(ID|VARIANT_ID)=steamos' /etc/os-release 2>/dev/null \
   || grep -q 'ID_LIKE=arch' /etc/os-release 2>/dev/null && \
      grep -q 'steamos\|steamdeck' /etc/os-release 2>/dev/null; then
    IS_STEAMOS=true
fi

IS_DEBIAN=false
if command -v dpkg >/dev/null 2>&1; then
    IS_DEBIAN=true
fi

# ── 1. System dependencies ───────────────────────────────────────────────────
echo ""
echo "▶ Checking system dependencies…"

MISSING=()
MISSING_FATAL=false

if ! command -v python3 >/dev/null 2>&1; then
    MISSING+=("python3")
    MISSING_FATAL=true
fi

# Clipboard tool: on Wayland we need wl-paste; on X11 we need xclip.
if [ -n "${WAYLAND_DISPLAY:-}" ] || [ "${XDG_SESSION_TYPE:-}" = "wayland" ]; then
    if ! command -v wl-paste >/dev/null 2>&1; then
        MISSING+=("wl-clipboard (provides wl-paste)")
        MISSING_FATAL=true
    fi
else
    if ! command -v xclip >/dev/null 2>&1; then
        MISSING+=("xclip")
        MISSING_FATAL=true
    fi
fi

if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo "  Missing: ${MISSING[*]}"
    echo ""
    if $IS_STEAMOS; then
        echo "  On SteamOS the rootfs is read-only. To install missing tools, either:"
        echo "    • Temporarily unlock: sudo steamos-readonly disable && sudo pacman -S wl-clipboard python && sudo steamos-readonly enable"
        echo "    • Or use distrobox (ships with SteamOS) to build/run from a container"
    else
        echo "  Debian/Ubuntu:  sudo apt install xclip wl-clipboard python3"
        echo "  Arch:           sudo pacman -S xclip wl-clipboard python"
    fi
    echo "  Rustup (cargo): curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo ""
    if $MISSING_FATAL; then
        exit 1
    fi
fi

# cargo is needed only if we are going to build; check separately below.
echo "  ✓ python3 and clipboard tool found"

# ── 2. ALSA dev headers (Debian/Ubuntu only — skip on SteamOS/Arch) ─────────
echo ""
if $IS_DEBIAN; then
    echo "▶ Ensuring ALSA dev headers are installed…"
    if ! dpkg -s libasound2-dev >/dev/null 2>&1; then
        echo "  Installing libasound2-dev…"
        sudo apt-get install -y libasound2-dev
    else
        echo "  ✓ libasound2-dev already installed"
    fi
elif $IS_STEAMOS; then
    echo "▶ Skipping ALSA dev-header install (SteamOS immutable rootfs)."
    echo "  libasound runtime is present; headers should be available for cargo build."
    echo "  If the build fails, see the cross-compile note at the top of this script."
else
    echo "▶ Skipping apt-based ALSA dev install (non-Debian system)."
    echo "  Ensure libasound2-dev / alsa-lib is installed before cargo build."
fi

# ── 2b. input group check (needed for rdev on Wayland) ──────────────────────
echo ""
echo "▶ Checking /dev/input group membership (required for rdev on Wayland)…"
if groups "$USER" | grep -qw input 2>/dev/null; then
    echo "  ✓ $USER is in the 'input' group"
else
    echo ""
    echo "  ⚠  $USER is NOT in the 'input' group."
    echo "     rdev reads /dev/input/event* to detect key presses."
    echo "     Without group membership the hotkey will be silently ignored."
    echo ""
    echo "     Fix (works on SteamOS even with immutable rootfs):"
    echo "       sudo usermod -aG input $USER"
    echo "     Then log out and back in, or reboot."
    echo ""
    # Offer to do it now
    if [ -t 0 ]; then   # only prompt when stdin is a terminal
        read -r -p "  Add $USER to 'input' group now? [y/N] " _reply
        case "$_reply" in
            [Yy]*)
                sudo usermod -aG input "$USER"
                echo "  ✓ Done. Log out and back in for the change to take effect."
                ;;
            *)
                echo "  Skipping. Remember to run: sudo usermod -aG input $USER"
                ;;
        esac
    fi
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

# ── 4. Download Piper voice model ───────────────────────────────────────────
echo ""
echo "▶ Downloading Piper voice model ($VOICE_NAME)…"
mkdir -p "$MODEL_DIR"

ONNX_FILE="$MODEL_DIR/$VOICE_NAME.onnx"
JSON_FILE="$MODEL_DIR/$VOICE_NAME.onnx.json"

if [ -f "$ONNX_FILE" ] && [ -f "$JSON_FILE" ]; then
    echo "  ✓ Model already downloaded"
else
    echo "  Downloading $VOICE_NAME.onnx…"
    curl -L --progress-bar -o "$ONNX_FILE" "$VOICE_URL"
    echo "  Downloading $VOICE_NAME.onnx.json…"
    curl -L --progress-bar -o "$JSON_FILE" "$VOICE_JSON_URL"
    echo "  ✓ Model downloaded to $MODEL_DIR"
fi

# ── 5. Build Rust binary ────────────────────────────────────────────────────
echo ""
PREBUILT="$SCRIPT_DIR/target/release/voice-speak"
if [ -f "$PREBUILT" ] && $IS_STEAMOS; then
    echo "▶ Using pre-built binary (SteamOS — skipping cargo build)."
    echo "  Found: $PREBUILT"
    echo "  To rebuild: cargo build --release (requires ALSA dev headers)"
elif command -v cargo >/dev/null 2>&1; then
    echo "▶ Building Rust binary (release mode)…"
    cd "$SCRIPT_DIR"
    cargo build --release
    echo "  ✓ Build complete"
else
    if [ -f "$PREBUILT" ]; then
        echo "▶ cargo not found — using existing pre-built binary."
        echo "  Found: $PREBUILT"
    else
        echo "  ERROR: cargo not found and no pre-built binary at $PREBUILT"
        echo "  Install rustup: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo "  Or cross-build on another machine and copy target/release/voice-speak here."
        exit 1
    fi
fi

# ── 6. Install to ~/.local/bin ───────────────────────────────────────────────
echo ""
echo "▶ Installing to $INSTALL_DIR…"
mkdir -p "$INSTALL_DIR"

cp "$PREBUILT" "$INSTALL_DIR/voice-speak"
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
echo ""
echo "Hotkey:             Right Alt (AltGr)"
echo "  Press once  →  speak highlighted text"
echo "  Press again →  stop playback"
echo ""
echo "To change the hotkey: edit src/main.rs → Config::hotkey"
echo "                      then re-run this installer."
