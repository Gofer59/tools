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
#   2. Installs runtime system packages via pacman
#   3. Re-locks the filesystem
#   4. Checks input group membership
#   5. Checks for pre-built binary
#   6. Creates Python venv + installs dependencies
#   7. Downloads Piper voice model
#   8. Installs files to ~/.local/bin/ and generates wrapper scripts
#   9. Installs KDE application menu entry + checks PATH

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

info "Step 1/9: Unlocking SteamOS read-only filesystem…"
warn "System packages installed via pacman may be wiped by a SteamOS major update."
warn "If that happens, re-run steps 1–3 of this installer (or the full script)."
warn "See the 'SteamOS update survival' section in README.md."
echo

sudo steamos-readonly disable
ok "Filesystem unlocked."
echo

# ─────────────────────────────────────────────────────────────────────────────
# Step 2 — Install runtime system packages
# ─────────────────────────────────────────────────────────────────────────────

info "Step 2/9: Installing runtime system packages via pacman…"
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
# Step 3 — Re-lock filesystem
# ─────────────────────────────────────────────────────────────────────────────

info "Step 3/9: Re-locking SteamOS filesystem…"
sudo steamos-readonly enable
ok "Filesystem re-locked."
echo

# ─────────────────────────────────────────────────────────────────────────────
# Step 4 — input group check
# ─────────────────────────────────────────────────────────────────────────────

info "Step 4/9: Checking input group membership…"
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
# Step 5 — Check for pre-built binary
# ─────────────────────────────────────────────────────────────────────────────

info "Step 5/9: Checking for pre-built binary…"
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
# Step 6 — Python venv + dependencies
# ─────────────────────────────────────────────────────────────────────────────

info "Step 6/9: Setting up Python venv at ${VENV_DIR}…"
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
# Step 7 — Download Piper voice model
# ─────────────────────────────────────────────────────────────────────────────

info "Step 7/9: Checking Piper voice model (${VOICE_NAME})…"
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
# Step 8 — Install files and generate wrapper scripts
# ─────────────────────────────────────────────────────────────────────────────

info "Step 8/9: Installing files to ${INSTALL_DIR}…"
mkdir -p "${INSTALL_DIR}"

# Binary
install -m 755 "${BUILT_BINARY}" "${INSTALL_DIR}/deck-reader"

# Python scripts
install -m 644 "${SCRIPT_DIR}/python/tts_speak.py"   "${INSTALL_DIR}/tts_speak.py"
install -m 644 "${SCRIPT_DIR}/python/tts_daemon.py"  "${INSTALL_DIR}/tts_daemon.py"
install -m 644 "${SCRIPT_DIR}/python/ocr_extract.py" "${INSTALL_DIR}/ocr_extract.py"
install -m 644 "${SCRIPT_DIR}/python/gui_window.py"  "${INSTALL_DIR}/gui_window.py"

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
# Step 9 — KDE application menu entry + PATH check
# ─────────────────────────────────────────────────────────────────────────────

info "Step 9/9: Installing KDE application menu entry…"
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

update-desktop-database "${APPS_DIR}" 2>/dev/null || true
ok "App menu entry written to ${APPS_DIR}/deck-reader.desktop"

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
