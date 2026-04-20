#!/usr/bin/env bash
# post-update-fix.sh — minimal recovery after a SteamOS major update.
#
# After a SteamOS update, two things break deck-reader:
#   1. System packages installed via pacman are wiped.
#   2. The pacman keyring at /etc/pacman.d/gnupg is reset / non-writable.
#
# This script re-initialises the keyring and re-installs the system packages
# deck-reader needs. Everything in ~/.local and ~/.config (binary, venv,
# models, config) survives updates untouched, so this is the only thing
# you need to run to make deck-reader work again.
#
# Launch by double-clicking "Deck Reader — Post-update fix" in the KDE
# application menu, or run directly:  ./post-update-fix.sh

set -euo pipefail

info()  { printf '\e[1;36m[deck-reader]\e[0m %s\n' "$*"; }
ok()    { printf '\e[1;32m[deck-reader]\e[0m %s\n' "$*"; }
warn()  { printf '\e[1;33m[WARN]\e[0m %s\n' "$*" >&2; }

info "SteamOS post-update recovery for deck-reader"
info "You will be prompted for your sudo password."
echo

info "Step 1/4: Unlocking SteamOS read-only filesystem…"
sudo steamos-readonly disable
ok "Filesystem unlocked."
echo

info "Step 2/4: Initialising pacman keyring…"
sudo pacman-key --init
sudo pacman-key --populate archlinux
sudo pacman-key --populate holo 2>/dev/null || \
    warn "holo keyring not populated (may be absent on this SteamOS image) — continuing."
ok "Keyring ready."
echo

info "Step 3/4: Installing runtime system packages via pacman…"
sudo pacman -S --noconfirm --needed \
    xclip \
    xdotool \
    maim \
    slop \
    wl-clipboard \
    grim \
    slurp \
    tesseract \
    tesseract-data-eng \
    tk
ok "System packages installed."
echo

info "Step 4/4: Re-locking SteamOS filesystem…"
sudo steamos-readonly enable
ok "Filesystem re-locked."
echo

ok "══════════════════════════════════════════════════"
ok "  deck-reader post-update recovery complete!"
ok "══════════════════════════════════════════════════"
echo
echo "  Launch:   search 'Deck Reader' in KDE app menu, or run: deck-reader"
echo
echo "Press Enter to close this window."
read -r _
