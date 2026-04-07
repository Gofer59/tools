#!/usr/bin/env bash
# install.sh — Build screen-ocr and install everything
#
# Run once:
#   chmod +x install.sh && ./install.sh
#
# After that just run:  screen-ocr
# (assuming ~/.local/bin is in your PATH — it is by default on Linux Mint)

set -euo pipefail

INSTALL_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.local/share/screen-ocr"
VENV_DIR="$DATA_DIR/venv"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "════════════════════════════════════════"
echo " screen-ocr installer"
echo "════════════════════════════════════════"

# ── 0. Detect package manager ───────────────────────────────────────────────
PKG_MANAGER=""
if command -v apt-get >/dev/null 2>&1; then
    PKG_MANAGER="apt"
elif command -v pacman >/dev/null 2>&1; then
    PKG_MANAGER="pacman"
else
    echo "⚠ Could not detect apt or pacman. You may need to install"
    echo "  system packages manually."
    PKG_MANAGER="unknown"
fi
echo ""
echo "▶ Detected package manager: $PKG_MANAGER"

# ── 1. System dependencies ──────────────────────────────────────────────────
echo ""
echo "▶ Checking system dependencies…"

MISSING=()
command -v cargo   >/dev/null 2>&1 || MISSING+=("cargo")
command -v python3 >/dev/null 2>&1 || MISSING+=("python3")

if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo "  Missing: ${MISSING[*]}"
    echo "  Install cargo with:"
    echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "  ✓ cargo, python3 found"

# ── 2. Display server + capture/clipboard tools ─────────────────────────────
echo ""
echo "▶ Detecting display server…"

DISPLAY_SERVER="x11"
if [ "${XDG_SESSION_TYPE:-}" = "wayland" ] || [ -n "${WAYLAND_DISPLAY:-}" ]; then
    DISPLAY_SERVER="wayland"
fi
echo "  Display server: $DISPLAY_SERVER"

# Install screen capture + clipboard tools based on display server
echo ""
echo "▶ Ensuring screen capture and clipboard tools are installed…"

if [ "$DISPLAY_SERVER" = "x11" ]; then
    # X11: maim (region capture with built-in slop), xclip, xdotool
    NEED_PKGS=()
    command -v maim    >/dev/null 2>&1 || NEED_PKGS+=("maim")
    command -v slop    >/dev/null 2>&1 || NEED_PKGS+=("slop")
    command -v xclip   >/dev/null 2>&1 || NEED_PKGS+=("xclip")
    command -v xdotool >/dev/null 2>&1 || NEED_PKGS+=("xdotool")

    if [[ ${#NEED_PKGS[@]} -gt 0 ]]; then
        echo "  Installing: ${NEED_PKGS[*]}"
        if [ "$PKG_MANAGER" = "apt" ]; then
            sudo apt-get install -y "${NEED_PKGS[@]}"
        elif [ "$PKG_MANAGER" = "pacman" ]; then
            sudo pacman -S --noconfirm "${NEED_PKGS[@]}"
        else
            echo "  ⚠ Please install manually: ${NEED_PKGS[*]}"
        fi
    fi
    echo "  ✓ maim, slop, xclip, xdotool available"

else
    # Wayland: grim (screenshot), slurp (region select), wl-clipboard
    NEED_PKGS=()
    command -v grim     >/dev/null 2>&1 || NEED_PKGS+=("grim")
    command -v slurp    >/dev/null 2>&1 || NEED_PKGS+=("slurp")
    command -v wl-copy  >/dev/null 2>&1 || NEED_PKGS+=("wl-clipboard")

    if [[ ${#NEED_PKGS[@]} -gt 0 ]]; then
        echo "  Installing: ${NEED_PKGS[*]}"
        if [ "$PKG_MANAGER" = "apt" ]; then
            sudo apt-get install -y "${NEED_PKGS[@]}"
        elif [ "$PKG_MANAGER" = "pacman" ]; then
            sudo pacman -S --noconfirm "${NEED_PKGS[@]}"
        else
            echo "  ⚠ Please install manually: ${NEED_PKGS[*]}"
        fi
    fi
    echo "  ✓ grim, slurp, wl-clipboard available"
fi

# ── 3. Tesseract OCR engine ─────────────────────────────────────────────────
echo ""
echo "▶ Ensuring Tesseract OCR is installed…"

if ! command -v tesseract >/dev/null 2>&1; then
    echo "  Installing Tesseract…"
    if [ "$PKG_MANAGER" = "apt" ]; then
        sudo apt-get install -y tesseract-ocr
    elif [ "$PKG_MANAGER" = "pacman" ]; then
        sudo pacman -S --noconfirm tesseract tesseract-data-eng
    else
        echo "  ⚠ Please install Tesseract manually:"
        echo "    Debian/Ubuntu: sudo apt install tesseract-ocr"
        echo "    Arch/SteamOS:  sudo pacman -S tesseract tesseract-data-eng"
    fi
else
    echo "  ✓ tesseract already installed"
fi

# ── 3b. TTS (voice-speak) check ──────────────────────────────────────────────
echo ""
echo "▶ Checking for voice-speak (TTS)…"
if [ -x "$HOME/.local/bin/tts_speak_wrapper.sh" ]; then
    echo "  ✓ tts_speak_wrapper.sh found — TTS enabled"
else
    echo "  ⚠ voice-speak not installed — TTS will be unavailable."
    echo "    To enable text-to-speech, install voice-speak first:"
    echo "      cd ../voice-speak && ./install.sh"
fi

# ── 4. ALSA development headers (needed by rdev) ────────────────────────────
echo ""
echo "▶ Ensuring ALSA dev headers are installed…"

if [ "$PKG_MANAGER" = "apt" ]; then
    if ! dpkg -s libasound2-dev >/dev/null 2>&1; then
        echo "  Installing libasound2-dev…"
        sudo apt-get install -y libasound2-dev
    else
        echo "  ✓ libasound2-dev already installed"
    fi
elif [ "$PKG_MANAGER" = "pacman" ]; then
    if ! pacman -Qi alsa-lib >/dev/null 2>&1; then
        echo "  Installing alsa-lib…"
        sudo pacman -S --noconfirm alsa-lib
    else
        echo "  ✓ alsa-lib already installed"
    fi
else
    echo "  ⚠ Ensure ALSA development headers are installed (needed to build rdev)"
fi

# ── 5. Python virtual environment + dependencies ────────────────────────────
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
echo "  ✓ Python dependencies installed (pytesseract, Pillow)"

# ── 6. Build Rust binary ────────────────────────────────────────────────────
echo ""
echo "▶ Building Rust binary (release mode)…"
cd "$SCRIPT_DIR"
cargo build --release
echo "  ✓ Build complete"

# ── 7. Install to ~/.local/bin ───────────────────────────────────────────────
echo ""
echo "▶ Installing to $INSTALL_DIR…"
mkdir -p "$INSTALL_DIR"

cp target/release/screen-ocr "$INSTALL_DIR/screen-ocr"
cp python/ocr_extract.py "$INSTALL_DIR/ocr_extract.py"
chmod +x "$INSTALL_DIR/screen-ocr"
chmod +x "$INSTALL_DIR/ocr_extract.py"

# Create a wrapper that activates the venv before running the Python script.
cat > "$INSTALL_DIR/ocr_extract_wrapper.sh" << WRAPPER
#!/usr/bin/env bash
# Auto-generated wrapper that runs ocr_extract.py inside the screen-ocr venv.
exec "$VENV_DIR/bin/python3" "$INSTALL_DIR/ocr_extract.py" "\$@"
WRAPPER
chmod +x "$INSTALL_DIR/ocr_extract_wrapper.sh"

echo "  ✓ screen-ocr            → $INSTALL_DIR/screen-ocr"
echo "  ✓ ocr_extract.py        → $INSTALL_DIR/ocr_extract.py"
echo "  ✓ wrapper script        → $INSTALL_DIR/ocr_extract_wrapper.sh"

# ── 8. PATH check ───────────────────────────────────────────────────────────
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

# ── 9. Done ──────────────────────────────────────────────────────────────────
echo ""
echo "════════════════════════════════════════"
echo " Installation complete!"
echo "════════════════════════════════════════"
echo ""
echo "Run manually:       screen-ocr"
echo ""
echo "Hotkeys:"
echo "  F9    Quick capture (re-use last selected region)"
echo "  F10   Select new region (interactive rectangle)"
echo ""
echo "Both run: capture → OCR → clipboard → TTS (speak aloud)"
echo ""
echo "Visual novel workflow:"
echo "  1. Press F10 to select the dialogue text box once"
echo "  2. Press F9 for each new line of dialogue"
echo ""
echo "To change hotkeys: edit src/main.rs → Config"
echo "                   then re-run this installer."
echo ""
echo "SteamDeck note: In Desktop Mode (Wayland), grim+slurp"
echo "  handle screen capture. Game Mode (Gamescope) may not"
echo "  forward hotkeys — use Desktop Mode for best results."
