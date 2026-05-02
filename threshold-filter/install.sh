#!/usr/bin/env bash
# install.sh — Build threshold-filter (Tauri + egui overlay) and install
#
# Run once:
#   chmod +x install.sh && ./install.sh
#
# After that just run:  threshold-filter
# (assuming ~/.local/bin is in your PATH — it is by default on Linux Mint)

set -euo pipefail

INSTALL_DIR="$HOME/.local/bin"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "════════════════════════════════════════"
echo " threshold-filter installer"
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
command -v cargo >/dev/null 2>&1 || MISSING+=("cargo (install via https://sh.rustup.rs)")
command -v node  >/dev/null 2>&1 || MISSING+=("node (install via https://nodejs.org or nvm)")
command -v npm   >/dev/null 2>&1 || MISSING+=("npm")

if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo "  Missing required tools:"
    for m in "${MISSING[@]}"; do
        echo "    - $m"
    done
    exit 1
fi

echo "  ✓ cargo, node, npm found"

# ── 2. System build dependencies (Tauri) ────────────────────────────────────
echo ""
echo "▶ Ensuring system build dependencies are installed…"

if [ "$PKG_MANAGER" = "apt" ]; then
    NEED_PKGS=()
    dpkg -s libwebkit2gtk-4.1-dev         >/dev/null 2>&1 || NEED_PKGS+=("libwebkit2gtk-4.1-dev")
    dpkg -s libayatana-appindicator3-dev  >/dev/null 2>&1 || NEED_PKGS+=("libayatana-appindicator3-dev")
    dpkg -s libssl-dev                    >/dev/null 2>&1 || NEED_PKGS+=("libssl-dev")
    dpkg -s pkg-config                    >/dev/null 2>&1 || NEED_PKGS+=("pkg-config")
    dpkg -s build-essential               >/dev/null 2>&1 || NEED_PKGS+=("build-essential")

    if [[ ${#NEED_PKGS[@]} -gt 0 ]]; then
        echo "  Installing: ${NEED_PKGS[*]}"
        sudo apt-get install -y "${NEED_PKGS[@]}"
    fi
    echo "  ✓ Tauri build dependencies installed"

elif [ "$PKG_MANAGER" = "dnf" ]; then
    echo "  Installing Tauri build dependencies via dnf…"
    sudo dnf install -y webkit2gtk4.1-devel libayatana-appindicator-gtk3-devel \
        openssl-devel pkg-config gcc
    echo "  ✓ Tauri build dependencies installed"

elif [ "$PKG_MANAGER" = "pacman" ]; then
    echo "  Installing Tauri build dependencies via pacman…"
    sudo pacman -S --noconfirm webkit2gtk-4.1 libayatana-appindicator \
        openssl pkg-config base-devel
    echo "  ✓ Tauri build dependencies installed"

else
    echo "  ⚠ Please install manually:"
    echo "    - WebKit2GTK 4.1 dev headers (e.g. libwebkit2gtk-4.1-dev)"
    echo "    - libayatana-appindicator3-dev"
    echo "    - libssl-dev / openssl-devel"
    echo "    - pkg-config, build-essential"
fi

# ── 3. egui/X11 overlay dependencies ────────────────────────────────────────
echo ""
echo "▶ Ensuring egui/xcap overlay dependencies are installed…"

if [ "$PKG_MANAGER" = "apt" ]; then
    NEED_PKGS=()
    dpkg -s libx11-dev     >/dev/null 2>&1 || NEED_PKGS+=("libx11-dev")
    dpkg -s libxcursor-dev >/dev/null 2>&1 || NEED_PKGS+=("libxcursor-dev")
    dpkg -s libxrandr-dev  >/dev/null 2>&1 || NEED_PKGS+=("libxrandr-dev")
    dpkg -s libxi-dev      >/dev/null 2>&1 || NEED_PKGS+=("libxi-dev")
    dpkg -s libxcb1-dev    >/dev/null 2>&1 || NEED_PKGS+=("libxcb1-dev")
    dpkg -s libxcb-xfixes0-dev >/dev/null 2>&1 || NEED_PKGS+=("libxcb-xfixes0-dev")
    dpkg -s libxcb-shm0-dev    >/dev/null 2>&1 || NEED_PKGS+=("libxcb-shm0-dev")

    if [[ ${#NEED_PKGS[@]} -gt 0 ]]; then
        echo "  Installing: ${NEED_PKGS[*]}"
        sudo apt-get install -y "${NEED_PKGS[@]}"
    fi
    echo "  ✓ egui/xcap overlay dependencies installed"

elif [ "$PKG_MANAGER" = "dnf" ]; then
    sudo dnf install -y libX11-devel libXcursor-devel libXrandr-devel libXi-devel \
        libxcb-devel
    echo "  ✓ egui/xcap overlay dependencies installed"

elif [ "$PKG_MANAGER" = "pacman" ]; then
    sudo pacman -S --noconfirm libx11 libxcursor libxrandr libxi libxcb
    echo "  ✓ egui/xcap overlay dependencies installed"

else
    echo "  ⚠ Please install manually:"
    echo "    - libX11-dev, libXcursor-dev, libXrandr-dev, libXi-dev"
    echo "    - libxcb-dev, libxcb-xfixes0-dev, libxcb-shm0-dev"
fi

# ── 4. Region selection tool (slop for X11 region draw) ─────────────────────
echo ""
echo "▶ Ensuring region selection tool is available…"

if ! command -v slop >/dev/null 2>&1; then
    echo "  Installing slop (X11 region selection)…"
    if [ "$PKG_MANAGER" = "apt" ]; then
        sudo apt-get install -y slop
    elif [ "$PKG_MANAGER" = "dnf" ]; then
        sudo dnf install -y slop
    elif [ "$PKG_MANAGER" = "pacman" ]; then
        sudo pacman -S --noconfirm slop
    else
        echo "  ⚠ Please install slop manually (for X11 region selection)"
    fi
else
    echo "  ✓ slop already installed"
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

# ── 6. Build Tauri app ───────────────────────────────────────────────────────
echo ""
echo "▶ Building threshold-filter (Tauri + egui, release mode)…"
cd "$SCRIPT_DIR"

npm --prefix ui install
npm --prefix ui run build
cargo tauri build

echo "  ✓ Build complete"

# ── 7. Install binary ────────────────────────────────────────────────────────
echo ""
echo "▶ Installing to $INSTALL_DIR…"
mkdir -p "$INSTALL_DIR"

cp "src-tauri/target/release/threshold-filter" "$INSTALL_DIR/threshold-filter"
chmod +x "$INSTALL_DIR/threshold-filter"
echo "  ✓ threshold-filter → $INSTALL_DIR/threshold-filter"

# ── 8. input group (rdev) ──────────────────────────────────────────────────
echo ""
echo "▶ Ensuring user is in the 'input' group (required for hotkeys via rdev)…"
if groups "$USER" | grep -q '\binput\b'; then
    echo "  ✓ $USER is already in the input group"
else
    sudo usermod -aG input "$USER"
    echo "  ✓ Added $USER to the input group"
    echo "  ⚠ You must log out and back in (or reboot) for this change to take effect."
fi

# ── 9. PATH check ───────────────────────────────────────────────────────────
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

# ── 10. Done ──────────────────────────────────────────────────────────────────
echo ""
echo "════════════════════════════════════════"
echo " Installation complete!"
echo "════════════════════════════════════════"
echo ""
echo "Launch:  threshold-filter"
echo "         (opens settings UI; overlay starts automatically)"
echo ""
echo "Hotkeys (default):"
echo "  F10   Select new region (interactive rectangle)"
echo "  F9    Toggle always-on-top for the overlay window"
echo ""
echo "To change hotkeys or threshold defaults: open threshold-filter → Settings."
echo ""
echo "Note: The overlay (egui) is a separate process launched automatically."
echo "You can start/stop it from the Settings UI or the tray icon menu."
