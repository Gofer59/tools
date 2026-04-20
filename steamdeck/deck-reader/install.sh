#!/usr/bin/env bash
# install.sh — deck-reader installer for SteamDeck (SteamOS 3.x / KDE Plasma / Wayland)
#
# PREREQUISITE: Build on your dev machine first:
#   cd deck-reader && cargo build --release
# Then copy the entire deck-reader/ folder to the SteamDeck and run:
#   chmod +x install.sh
#   ./install.sh
#
# What this does (in order):
#   1. Unlocks the SteamOS read-only filesystem
#   2. Initializes/populates the pacman keyring (required after SteamOS updates)
#   3. Installs runtime system packages via pacman
#   4. Re-locks the filesystem
#   5. Checks input group membership
#   6. Checks for pre-built binary
#   7. Creates Python venv + installs dependencies
#   8. Downloads Piper voice model
#   9. Installs files to ~/.local/bin/ and generates wrapper scripts
#  10. Installs KDE application menu entry + checks PATH

set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Paths
# ─────────────────────────────────────────────────────────────────────────────

INSTALL_DIR="${HOME}/.local/bin"
DATA_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}/deck-reader"
VENV_DIR="${DATA_DIR}/venv"
MODELS_DIR="${DATA_DIR}/models"
CONFIG_DIR="${XDG_CONFIG_HOME:-${HOME}/.config}/deck-reader"
APPS_DIR="${HOME}/.local/share/applications"

VOICE_NAME="en_US-lessac-medium"
VOICE_BASE_URL="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/lessac/medium"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

info()  { printf '\e[1;36m[deck-reader]\e[0m %s\n' "$*"; }
ok()    { printf '\e[1;32m[deck-reader]\e[0m %s\n' "$*"; }
warn()  { printf '\e[1;33m[WARN]\e[0m %s\n' "$*" >&2; }
error() { printf '\e[1;31m[ERROR]\e[0m %s\n' "$*" >&2; }

ask_yes() {
    read -r -p "$1 [y/N] " reply
    [[ "${reply,,}" == "y" || "${reply,,}" == "yes" ]]
}

# ─────────────────────────────────────────────────────────────────────────────
# Step 1 — Unlock SteamOS read-only filesystem
# ─────────────────────────────────────────────────────────────────────────────

info "Step 1/10: Unlocking SteamOS read-only filesystem…"
warn "System packages installed via pacman may be wiped by a SteamOS major update."
warn "If that happens, re-run steps 1–4 of this installer (or the full script)."
warn "See the 'SteamOS update survival' section in README.md."
echo

sudo steamos-readonly disable
ok "Filesystem unlocked."
echo

# ─────────────────────────────────────────────────────────────────────────────
# Step 2 — Initialize / populate pacman keyring
# ─────────────────────────────────────────────────────────────────────────────
#
# After a SteamOS major update, /etc/pacman.d/gnupg is often wiped or made
# read-only, so pacman -S fails with:
#     warning: Public keyring not found; have you run 'pacman-key --init'?
#     error: keyring is not writable
#     error: required key missing from keyring
# Re-initialising and populating the keyring here makes the installer idempotent
# across SteamOS updates.

info "Step 2/10: Initialising pacman keyring…"
sudo pacman-key --init
# "holo" is the SteamOS-specific keyring; it may be absent on some images, so
# don't abort if only archlinux populates successfully.
sudo pacman-key --populate archlinux
sudo pacman-key --populate holo 2>/dev/null || \
    warn "holo keyring not populated (may be absent on this SteamOS image) — continuing."
ok "Keyring ready."
echo

# ─────────────────────────────────────────────────────────────────────────────
# Step 3 — Install runtime system packages
# ─────────────────────────────────────────────────────────────────────────────

info "Step 3/10: Installing runtime system packages via pacman…"
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

# ─────────────────────────────────────────────────────────────────────────────
# Step 4 — Re-lock filesystem
# ─────────────────────────────────────────────────────────────────────────────

info "Step 4/10: Re-locking SteamOS filesystem…"
sudo steamos-readonly enable
ok "Filesystem re-locked."
echo

# ─────────────────────────────────────────────────────────────────────────────
# Step 5 — input group check
# ─────────────────────────────────────────────────────────────────────────────

info "Step 5/10: Checking input group membership…"
if groups "${USER}" | grep -qw input; then
    ok "User '${USER}' is already in the 'input' group."
else
    warn "User '${USER}' is NOT in the 'input' group."
    warn "rdev (the global hotkey listener) requires this to capture key events."
    echo
    if ask_yes "Add ${USER} to the input group now? (requires sudo; reboot needed after)"; then
        sudo usermod -aG input "${USER}"
        ok "Added to input group. You must reboot before hotkeys will work."
    else
        warn "Skipped. Add manually with:  sudo usermod -aG input ${USER}  then reboot."
    fi
fi
echo

# ─────────────────────────────────────────────────────────────────────────────
# Step 6 — Check for pre-built binary
# ─────────────────────────────────────────────────────────────────────────────

info "Step 6/10: Checking for pre-built binary…"
BUILT_BINARY="${SCRIPT_DIR}/target/release/deck-reader"

if [[ ! -f "${BUILT_BINARY}" ]]; then
    error "Pre-built binary not found at:"
    error "  ${BUILT_BINARY}"
    echo
    echo "Build on your dev machine first (not on the SteamDeck):"
    echo "  cd deck-reader"
    echo "  cargo build --release"
    echo
    echo "Then copy the entire deck-reader/ folder to the SteamDeck"
    echo "and re-run ./install.sh"
    exit 1
fi
ok "Found pre-built binary."
echo

# ─────────────────────────────────────────────────────────────────────────────
# Step 7 — Python venv + dependencies
# ─────────────────────────────────────────────────────────────────────────────

info "Step 7/10: Setting up Python venv at ${VENV_DIR}…"
mkdir -p "${DATA_DIR}"

if [[ ! -d "${VENV_DIR}" ]]; then
    python3 -m venv "${VENV_DIR}"
    ok "Created venv."
else
    ok "Venv already exists, skipping creation."
fi

info "Installing Python dependencies…"
"${VENV_DIR}/bin/pip" install --upgrade pip --quiet
"${VENV_DIR}/bin/pip" install -r "${SCRIPT_DIR}/requirements.txt"
ok "Python dependencies installed."
echo

# ─────────────────────────────────────────────────────────────────────────────
# Step 8 — Download Piper voice model
# ─────────────────────────────────────────────────────────────────────────────

info "Step 8/10: Checking Piper voice model (${VOICE_NAME})…"
mkdir -p "${MODELS_DIR}"

ONNX_FILE="${MODELS_DIR}/${VOICE_NAME}.onnx"
JSON_FILE="${MODELS_DIR}/${VOICE_NAME}.onnx.json"

if [[ -f "${ONNX_FILE}" && -f "${JSON_FILE}" ]]; then
    ok "Model already present, skipping download."
else
    info "Downloading ${VOICE_NAME} from HuggingFace…"
    curl -L --progress-bar \
        -o "${ONNX_FILE}" \
        "${VOICE_BASE_URL}/${VOICE_NAME}.onnx"
    curl -L --progress-bar \
        -o "${JSON_FILE}" \
        "${VOICE_BASE_URL}/${VOICE_NAME}.onnx.json"
    ok "Model downloaded to ${MODELS_DIR}/"
fi
echo

# ─────────────────────────────────────────────────────────────────────────────
# Step 9 — Install files and generate wrapper scripts
# ─────────────────────────────────────────────────────────────────────────────

info "Step 9/10: Installing files to ${INSTALL_DIR}…"
mkdir -p "${INSTALL_DIR}"

# Binary
install -m 755 "${BUILT_BINARY}" "${INSTALL_DIR}/deck-reader"

# Python scripts
install -m 644 "${SCRIPT_DIR}/python/tts_speak.py"   "${INSTALL_DIR}/tts_speak.py"
install -m 644 "${SCRIPT_DIR}/python/tts_daemon.py"  "${INSTALL_DIR}/tts_daemon.py"
install -m 644 "${SCRIPT_DIR}/python/ocr_extract.py" "${INSTALL_DIR}/ocr_extract.py"
install -m 644 "${SCRIPT_DIR}/python/gui_window.py"  "${INSTALL_DIR}/gui_window.py"

# Post-update recovery script (survives SteamOS updates alongside the binary
# so the user can re-run it from the KDE app menu after any major update).
install -m 755 "${SCRIPT_DIR}/post-update-fix.sh" \
    "${INSTALL_DIR}/deck-reader-post-update-fix.sh"

# Auto-generated TTS wrapper (activates venv, then runs tts_speak.py)
cat > "${INSTALL_DIR}/tts_speak_wrapper.sh" << WRAPPER
#!/usr/bin/env bash
# Auto-generated by deck-reader install.sh — do not edit by hand.
exec "${VENV_DIR}/bin/python3" "${INSTALL_DIR}/tts_speak.py" "\$@"
WRAPPER
chmod +x "${INSTALL_DIR}/tts_speak_wrapper.sh"

# Auto-generated OCR wrapper (activates venv, then runs ocr_extract.py)
cat > "${INSTALL_DIR}/ocr_extract_wrapper.sh" << WRAPPER
#!/usr/bin/env bash
# Auto-generated by deck-reader install.sh — do not edit by hand.
exec "${VENV_DIR}/bin/python3" "${INSTALL_DIR}/ocr_extract.py" "\$@"
WRAPPER
chmod +x "${INSTALL_DIR}/ocr_extract_wrapper.sh"

ok "Files installed."
echo

# ─────────────────────────────────────────────────────────────────────────────
# Step 10 — KDE application menu entry + PATH check
# ─────────────────────────────────────────────────────────────────────────────

info "Step 10/10: Installing KDE application menu entry…"
mkdir -p "${APPS_DIR}"

cat > "${APPS_DIR}/deck-reader.desktop" << DESKTOP
[Desktop Entry]
Type=Application
Name=Deck Reader
Comment=Screen OCR + TTS for visual novels
Exec=${INSTALL_DIR}/deck-reader
Icon=utilities-terminal
Terminal=false
Categories=Utility;Accessibility;
Keywords=ocr;tts;speech;screen;reader;
DESKTOP

# Post-update recovery launcher — appears in the KDE application menu so the
# user can click one entry to restore deck-reader after a SteamOS major update.
cat > "${APPS_DIR}/deck-reader-post-update-fix.desktop" << DESKTOP
[Desktop Entry]
Type=Application
Name=Deck Reader — Post-update fix
GenericName=SteamOS post-update recovery
Comment=Reinstall deck-reader's system dependencies after a SteamOS major update
Exec=konsole --hold -e ${INSTALL_DIR}/deck-reader-post-update-fix.sh
Icon=system-software-update
Terminal=false
Categories=Utility;System;
Keywords=deck-reader;pacman;keyring;steamos;update;recovery;
DESKTOP

update-desktop-database "${APPS_DIR}" 2>/dev/null || true
ok "App menu entries written:"
ok "  ${APPS_DIR}/deck-reader.desktop"
ok "  ${APPS_DIR}/deck-reader-post-update-fix.desktop"

# PATH check
if ! echo "${PATH}" | tr ':' '\n' | grep -qF "${INSTALL_DIR}"; then
    echo
    warn "${INSTALL_DIR} is NOT in your PATH."
    echo
    echo "Add it by appending to ~/.bashrc or ~/.bash_profile:"
    echo "  export PATH=\"\${HOME}/.local/bin:\${PATH}\""
    echo "Then run:  source ~/.bashrc"
fi
echo

# ─────────────────────────────────────────────────────────────────────────────
# Done
# ─────────────────────────────────────────────────────────────────────────────

ok "══════════════════════════════════════════════════"
ok "  deck-reader installed successfully!"
ok "══════════════════════════════════════════════════"
echo
echo "  Launch:     search 'Deck Reader' in KDE app menu, or run: deck-reader"
echo "  Config:     ${CONFIG_DIR}/config.toml"
echo "  Hotkeys:    Alt+U       select region + OCR"
echo "              Alt+I       re-capture + OCR"
echo "              Alt+Y       TTS toggle"
echo
if ! groups "${USER}" | grep -qw input; then
    warn "REMINDER: Reboot to activate input group membership before hotkeys work."
fi
