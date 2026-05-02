#!/usr/bin/env bash
# install.sh — Build screen-ocr (Tauri) and install everything
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
echo " screen-ocr installer (Tauri)"
echo "════════════════════════════════════════"

# ── 0. Detect package manager ───────────────────────────────────────────────
PKG_MANAGER=""
if command -v apt-get >/dev/null 2>&1; then
    PKG_MANAGER="apt"
elif command -v dnf >/dev/null 2>&1; then
    PKG_MANAGER="dnf"
elif command -v pacman >/dev/null 2>&1; then
    PKG_MANAGER="pacman"
else
    echo "⚠ Could not detect apt, dnf, or pacman. You may need to install"
    echo "  system packages manually."
    PKG_MANAGER="unknown"
fi
echo ""
echo "▶ Detected package manager: $PKG_MANAGER"

# ── 1. Check required tools ─────────────────────────────────────────────────
echo ""
echo "▶ Checking required tools…"

MISSING=()
command -v cargo   >/dev/null 2>&1 || MISSING+=("cargo (install via https://sh.rustup.rs)")
command -v python3 >/dev/null 2>&1 || MISSING+=("python3")
command -v node    >/dev/null 2>&1 || MISSING+=("node (install via https://nodejs.org or nvm)")
command -v npm     >/dev/null 2>&1 || MISSING+=("npm")

if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo "  Missing required tools:"
    for m in "${MISSING[@]}"; do
        echo "    - $m"
    done
    exit 1
fi

echo "  ✓ cargo, python3, node, npm found"

# ── 2. System build dependencies (Tauri) ────────────────────────────────────
echo ""
echo "▶ Ensuring system build dependencies are installed…"

if [ "$PKG_MANAGER" = "apt" ]; then
    NEED_PKGS=()
    dpkg -s libwebkit2gtk-4.1-dev          >/dev/null 2>&1 || NEED_PKGS+=("libwebkit2gtk-4.1-dev")
    dpkg -s libayatana-appindicator3-dev   >/dev/null 2>&1 || NEED_PKGS+=("libayatana-appindicator3-dev")
    dpkg -s libssl-dev                     >/dev/null 2>&1 || NEED_PKGS+=("libssl-dev")
    dpkg -s pkg-config                     >/dev/null 2>&1 || NEED_PKGS+=("pkg-config")
    dpkg -s build-essential                >/dev/null 2>&1 || NEED_PKGS+=("build-essential")
    dpkg -s libasound2-dev                 >/dev/null 2>&1 || NEED_PKGS+=("libasound2-dev")

    if [[ ${#NEED_PKGS[@]} -gt 0 ]]; then
        echo "  Installing: ${NEED_PKGS[*]}"
        sudo apt-get install -y "${NEED_PKGS[@]}"
    fi
    echo "  ✓ Tauri build dependencies installed"

elif [ "$PKG_MANAGER" = "dnf" ]; then
    echo "  Installing Tauri build dependencies via dnf…"
    sudo dnf install -y webkit2gtk4.1-devel libayatana-appindicator-gtk3-devel \
        openssl-devel pkg-config gcc alsa-lib-devel
    echo "  ✓ Tauri build dependencies installed"

elif [ "$PKG_MANAGER" = "pacman" ]; then
    echo "  Installing Tauri build dependencies via pacman…"
    sudo pacman -S --noconfirm webkit2gtk-4.1 libayatana-appindicator \
        openssl pkg-config base-devel alsa-lib
    echo "  ✓ Tauri build dependencies installed"

else
    echo "  ⚠ Please install manually:"
    echo "    - WebKit2GTK 4.1 dev headers (e.g. libwebkit2gtk-4.1-dev)"
    echo "    - libayatana-appindicator3-dev"
    echo "    - libssl-dev / openssl-devel"
    echo "    - pkg-config, build-essential"
    echo "    - libasound2-dev / alsa-lib-devel"
fi

# ── 3. Display server + capture/clipboard tools ─────────────────────────────
echo ""
echo "▶ Detecting display server…"

DISPLAY_SERVER="x11"
if [ "${XDG_SESSION_TYPE:-}" = "wayland" ] || [ -n "${WAYLAND_DISPLAY:-}" ]; then
    DISPLAY_SERVER="wayland"
fi
echo "  Display server: $DISPLAY_SERVER"

echo ""
echo "▶ Ensuring screen capture and clipboard tools are installed…"

if [ "$DISPLAY_SERVER" = "x11" ]; then
    # X11: maim (screenshot), slop (region select), xclip, xdotool
    NEED_PKGS=()
    command -v maim    >/dev/null 2>&1 || NEED_PKGS+=("maim")
    command -v slop    >/dev/null 2>&1 || NEED_PKGS+=("slop")
    command -v xclip   >/dev/null 2>&1 || NEED_PKGS+=("xclip")
    command -v xdotool >/dev/null 2>&1 || NEED_PKGS+=("xdotool")

    if [[ ${#NEED_PKGS[@]} -gt 0 ]]; then
        echo "  Installing: ${NEED_PKGS[*]}"
        if [ "$PKG_MANAGER" = "apt" ]; then
            sudo apt-get install -y "${NEED_PKGS[@]}"
        elif [ "$PKG_MANAGER" = "dnf" ]; then
            sudo dnf install -y "${NEED_PKGS[@]}"
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
    command -v grim    >/dev/null 2>&1 || NEED_PKGS+=("grim")
    command -v slurp   >/dev/null 2>&1 || NEED_PKGS+=("slurp")
    command -v wl-copy >/dev/null 2>&1 || NEED_PKGS+=("wl-clipboard")

    if [[ ${#NEED_PKGS[@]} -gt 0 ]]; then
        echo "  Installing: ${NEED_PKGS[*]}"
        if [ "$PKG_MANAGER" = "apt" ]; then
            sudo apt-get install -y "${NEED_PKGS[@]}"
        elif [ "$PKG_MANAGER" = "dnf" ]; then
            sudo dnf install -y "${NEED_PKGS[@]}"
        elif [ "$PKG_MANAGER" = "pacman" ]; then
            sudo pacman -S --noconfirm "${NEED_PKGS[@]}"
        else
            echo "  ⚠ Please install manually: ${NEED_PKGS[*]}"
        fi
    fi
    echo "  ✓ grim, slurp, wl-clipboard available"

    # ydotool for Wayland text injection
    if ! command -v ydotool >/dev/null 2>&1; then
        echo "  ⚠ ydotool not found — text injection on Wayland may be unavailable."
        echo "    Install via your package manager: ydotool"
    else
        echo "  ✓ ydotool available"
    fi
fi

# ── 4. Tesseract OCR engine ─────────────────────────────────────────────────
echo ""
echo "▶ Ensuring Tesseract OCR is installed…"

if ! command -v tesseract >/dev/null 2>&1; then
    echo "  Installing Tesseract…"
    if [ "$PKG_MANAGER" = "apt" ]; then
        sudo apt-get install -y tesseract-ocr tesseract-ocr-eng tesseract-ocr-fra
    elif [ "$PKG_MANAGER" = "dnf" ]; then
        sudo dnf install -y tesseract tesseract-langpack-eng tesseract-langpack-fra
    elif [ "$PKG_MANAGER" = "pacman" ]; then
        sudo pacman -S --noconfirm tesseract tesseract-data-eng tesseract-data-fra
    else
        echo "  ⚠ Please install Tesseract manually:"
        echo "    Debian/Ubuntu: sudo apt install tesseract-ocr"
        echo "    Fedora:        sudo dnf install tesseract"
        echo "    Arch/SteamOS:  sudo pacman -S tesseract tesseract-data-eng"
    fi
else
    echo "  ✓ tesseract already installed"
fi

# ── 5. cargo-tauri CLI ───────────────────────────────────────────────────────
echo ""
echo "▶ Checking for cargo-tauri CLI…"

if ! cargo tauri --version >/dev/null 2>&1; then
    echo "  cargo-tauri not found — installing (this may take a few minutes)…"
    cargo install tauri-cli --version "^2"
    echo "  ✓ cargo-tauri installed"
else
    echo "  ✓ cargo-tauri already installed ($(cargo tauri --version 2>/dev/null))"
fi

# ── 6. Python virtual environment + dependencies ────────────────────────────
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
"$VENV_DIR/bin/pip" install --quiet -r "$SCRIPT_DIR/python/requirements.txt"
echo "  ✓ Python dependencies installed (pytesseract, Pillow)"

# ── 7. Build Tauri app ───────────────────────────────────────────────────────
echo ""
echo "▶ Building screen-ocr (Tauri, release mode)…"
cd "$SCRIPT_DIR"

npm --prefix ui install
npm --prefix ui run build
cargo tauri build --manifest-path src-tauri/Cargo.toml

echo "  ✓ Build complete"

# ── 8. Install binary ────────────────────────────────────────────────────────
echo ""
echo "▶ Installing to $INSTALL_DIR…"
mkdir -p "$INSTALL_DIR"

cp "src-tauri/target/release/screen-ocr" "$INSTALL_DIR/screen-ocr"
chmod +x "$INSTALL_DIR/screen-ocr"
echo "  ✓ screen-ocr → $INSTALL_DIR/screen-ocr"

# ── 9. Install Python files ──────────────────────────────────────────────────
echo ""
echo "▶ Installing Python OCR backend…"
mkdir -p "$DATA_DIR"

cp "$SCRIPT_DIR/python/ocr_extract.py" "$DATA_DIR/ocr_extract.py"
chmod +x "$DATA_DIR/ocr_extract.py"

cat > "$DATA_DIR/ocr_extract_wrapper.sh" << WRAPPER
#!/usr/bin/env bash
# Auto-generated wrapper that runs ocr_extract.py inside the screen-ocr venv.
exec "$VENV_DIR/bin/python3" "$DATA_DIR/ocr_extract.py" "\$@"
WRAPPER
chmod +x "$DATA_DIR/ocr_extract_wrapper.sh"

echo "  ✓ ocr_extract.py        → $DATA_DIR/ocr_extract.py"
echo "  ✓ ocr_extract_wrapper   → $DATA_DIR/ocr_extract_wrapper.sh"

# ── 10. input group (rdev) ──────────────────────────────────────────────────
echo ""
echo "▶ Ensuring user is in the 'input' group (required for hotkeys via rdev)…"
if groups "$USER" | grep -q '\binput\b'; then
    echo "  ✓ $USER is already in the input group"
else
    sudo usermod -aG input "$USER"
    echo "  ✓ Added $USER to the input group"
    echo "  ⚠ You must log out and back in (or reboot) for this change to take effect."
fi

# ── 11. PATH check ───────────────────────────────────────────────────────────
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

# ── 12. Done ─────────────────────────────────────────────────────────────────
echo ""
echo "════════════════════════════════════════"
echo " Installation complete!"
echo "════════════════════════════════════════"
echo ""
echo "Launch:  screen-ocr"
echo "         (opens the settings/tray GUI)"
echo ""
echo "Hotkeys (default):"
echo "  F10   Select new region (interactive rectangle)"
echo "  F9    Quick re-capture (re-use last selected region)"
echo ""
echo "Both hotkeys: capture → OCR → clipboard → TTS (if voice-speak is installed)"
echo ""
echo "To change hotkeys or behaviour: open screen-ocr → Settings."
echo ""
echo "SteamDeck note: In Desktop Mode (Wayland), grim+slurp"
echo "  handle screen capture. Game Mode (Gamescope) may not"
echo "  forward hotkeys — use Desktop Mode for best results."
