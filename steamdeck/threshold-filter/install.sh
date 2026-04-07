#!/usr/bin/env bash
# install.sh — threshold-filter-deck installer for SteamDeck (SteamOS 3.x / Wayland)
#
# Build on your dev machine first:
#   cd threshold-filter/steamdeck && cargo build --release
# Then copy the steamdeck/ folder to the SteamDeck and run:
#   chmod +x install.sh && ./install.sh

set -euo pipefail

# Re-lock filesystem on any failure after unlocking
UNLOCKED=false
cleanup() { $UNLOCKED && sudo steamos-readonly enable 2>/dev/null && echo "Filesystem re-locked."; }
trap cleanup EXIT

INSTALL_DIR="${HOME}/.local/bin"
CONFIG_DIR="${XDG_CONFIG_HOME:-${HOME}/.config}/threshold-filter"
APPS_DIR="${HOME}/.local/share/applications"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

info()  { printf '\e[1;36m[threshold-filter]\e[0m %s\n' "$*"; }
ok()    { printf '\e[1;32m[threshold-filter]\e[0m %s\n' "$*"; }
warn()  { printf '\e[1;33m[WARN]\e[0m %s\n' "$*" >&2; }

ask_yes() {
    read -r -p "$1 [y/N] " reply
    [[ "${reply,,}" == "y" || "${reply,,}" == "yes" ]]
}

echo "════════════════════════════════════════"
echo " threshold-filter-deck installer"
echo "════════════════════════════════════════"
echo

# Step 1: Unlock SteamOS filesystem
info "Step 1/5: Unlocking SteamOS read-only filesystem..."
sudo steamos-readonly disable
UNLOCKED=true
ok "Filesystem unlocked."
echo

# Step 2: Install runtime packages
info "Step 2/5: Installing runtime packages via pacman..."
sudo pacman -S --noconfirm --needed grim slurp maim slop xdotool
ok "Packages installed."
echo

# Step 3: Re-lock filesystem
info "Step 3/5: Re-locking SteamOS filesystem..."
sudo steamos-readonly enable
UNLOCKED=false
ok "Filesystem re-locked."
echo

# Step 4: Input group check
info "Step 4/5: Checking input group membership..."
if groups "${USER}" | grep -qw input; then
    ok "User '${USER}' is already in the 'input' group."
else
    warn "User '${USER}' is NOT in the 'input' group."
    warn "rdev (the global hotkey listener) requires this."
    echo
    if ask_yes "Add ${USER} to the input group now? (requires sudo; reboot needed)"; then
        sudo usermod -aG input "${USER}"
        ok "Added to input group. Reboot before hotkeys will work."
    else
        warn "Skipped. Add manually with:  sudo usermod -aG input ${USER}  then reboot."
    fi
fi
echo

# Step 5: Install binary + menu entry
info "Step 5/5: Installing binary and menu entry..."
BUILT_BINARY="${SCRIPT_DIR}/target/release/threshold-filter-deck"

if [[ ! -f "${BUILT_BINARY}" ]]; then
    if command -v cargo >/dev/null 2>&1; then
        info "Binary not found — building with cargo..."
        (cd "${SCRIPT_DIR}" && cargo build --release)
        ok "Build complete."
    else
        echo "Pre-built binary not found at:"
        echo "  ${BUILT_BINARY}"
        echo
        echo "Build on your dev machine first:"
        echo "  cd threshold-filter/steamdeck"
        echo "  cargo build --release"
        echo
        echo "Then copy to the SteamDeck and re-run ./install.sh"
        exit 1
    fi
fi

mkdir -p "${INSTALL_DIR}"
install -m 755 "${BUILT_BINARY}" "${INSTALL_DIR}/threshold-filter-deck"

mkdir -p "${APPS_DIR}"
cat > "${APPS_DIR}/threshold-filter-deck.desktop" << DESKTOP
[Desktop Entry]
Type=Application
Name=Threshold Filter (Deck)
Comment=Screen threshold filter overlay for SteamDeck
Exec=${INSTALL_DIR}/threshold-filter-deck
Icon=utilities-terminal
Terminal=false
Categories=Utility;Graphics;
DESKTOP

update-desktop-database "${APPS_DIR}" 2>/dev/null || true

echo
ok "════════════════════════════════════════"
ok " threshold-filter-deck installed!"
ok "════════════════════════════════════════"
echo
echo "  Launch:     search 'Threshold Filter' in KDE app menu"
echo "  Config:     ${CONFIG_DIR}/config.toml"
echo "  Hotkeys:    F10  select window + region"
echo "              F8   toggle always-on-top"
echo
if ! echo "${PATH}" | tr ':' '\n' | grep -qF "${INSTALL_DIR}"; then
    warn "${INSTALL_DIR} is NOT in your PATH."
    echo "  Add to ~/.bashrc:  export PATH=\"\${HOME}/.local/bin:\${PATH}\""
    echo
fi

warn "NOTE: pacman packages (grim, slurp) don't survive SteamOS updates."
warn "Re-run install.sh after a SteamOS update."
