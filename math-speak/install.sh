#!/usr/bin/env bash
set -euo pipefail

TOOL="math-speak"
DATA="$HOME/.local/share/$TOOL"
MODELS="$DATA/models"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "[$TOOL] install starting"

# 1. System deps — only install what's actually missing, so a third-party
# nodejs/npm (nodesource, nvm, asdf, conda) doesn't fight the distro packages.

need_pkg () {
    # need_pkg <command> <apt-pkg> <dnf-pkg> <pacman-pkg>
    local cmd="$1" apt_p="$2" dnf_p="$3" pac_p="$4"
    if command -v "$cmd" >/dev/null 2>&1; then
        return 0
    fi
    case "$PKG_MGR" in
        apt)    APT_PKGS+=("$apt_p") ;;
        dnf)    DNF_PKGS+=("$dnf_p") ;;
        pacman) PAC_PKGS+=("$pac_p") ;;
    esac
}

need_lib_apt () {
    # apt-only: ensure a -dev/library package is present (no command to test).
    dpkg -s "$1" >/dev/null 2>&1 || APT_PKGS+=("$1")
}

PKG_MGR=""
if command -v apt-get >/dev/null; then PKG_MGR=apt
elif command -v dnf >/dev/null; then PKG_MGR=dnf
elif command -v pacman >/dev/null; then PKG_MGR=pacman
fi

APT_PKGS=()
DNF_PKGS=()
PAC_PKGS=()

# nodejs + npm are checked together: only install distro nodejs/npm if BOTH
# are missing. If node is already on PATH (e.g. from nodesource), assume npm
# came with it.
if ! command -v node >/dev/null 2>&1; then
    case "$PKG_MGR" in
        apt)    APT_PKGS+=("nodejs" "npm") ;;
        dnf)    DNF_PKGS+=("nodejs" "npm") ;;
        pacman) PAC_PKGS+=("nodejs" "npm") ;;
    esac
fi
if ! command -v npm >/dev/null 2>&1 && ! command -v node >/dev/null 2>&1; then
    : # already covered above
fi

need_pkg xclip      xclip          xclip          xclip
need_pkg wl-paste   wl-clipboard   wl-clipboard   wl-clipboard
need_pkg espeak-ng  espeak-ng      espeak-ng      espeak-ng
need_pkg pipx       pipx           pipx           python-pipx

case "$PKG_MGR" in
    apt)
        need_lib_apt libportaudio2
        need_lib_apt python3-venv
        need_lib_apt python3-pip
        if (( ${#APT_PKGS[@]} > 0 )); then
            echo "[$TOOL] apt installing: ${APT_PKGS[*]}"
            sudo apt-get update -qq
            sudo apt-get install -y "${APT_PKGS[@]}"
        else
            echo "[$TOOL] all system deps already present"
        fi
        ;;
    dnf)
        # portaudio is a -devel-style library; install if no command depends on it
        rpm -q portaudio >/dev/null 2>&1 || DNF_PKGS+=("portaudio")
        rpm -q python3-virtualenv >/dev/null 2>&1 || DNF_PKGS+=("python3-virtualenv")
        rpm -q python3-pip >/dev/null 2>&1 || DNF_PKGS+=("python3-pip")
        if (( ${#DNF_PKGS[@]} > 0 )); then
            echo "[$TOOL] dnf installing: ${DNF_PKGS[*]}"
            sudo dnf install -y "${DNF_PKGS[@]}"
        else
            echo "[$TOOL] all system deps already present"
        fi
        ;;
    pacman)
        pacman -Q portaudio >/dev/null 2>&1 || PAC_PKGS+=("portaudio")
        if (( ${#PAC_PKGS[@]} > 0 )); then
            echo "[$TOOL] pacman installing: ${PAC_PKGS[*]}"
            sudo pacman -S --needed --noconfirm "${PAC_PKGS[@]}"
        else
            echo "[$TOOL] all system deps already present"
        fi
        ;;
    *)
        echo "[$TOOL] no supported package manager; ensure these are present:"
        echo "        xclip wl-clipboard espeak-ng portaudio nodejs npm pipx"
        ;;
esac

# Verify the must-haves
for c in node npm pipx xclip espeak-ng; do
    if ! command -v "$c" >/dev/null 2>&1; then
        echo "[$TOOL] ERROR: '$c' is required but not on PATH" >&2
        exit 1
    fi
done

# 2. Node deps for SRE daemon (installed into ~/.local/share/math-speak/node)
echo "[$TOOL] installing speech-rule-engine + temml"
mkdir -p "$DATA/node"
cp "$SCRIPT_DIR/node/package.json" "$DATA/node/package.json"
cp "$SCRIPT_DIR/node/sre_daemon.js" "$DATA/node/sre_daemon.js"
( cd "$DATA/node" && npm install --omit=dev --no-fund --no-audit )

# 3. Python install via pipx
echo "[$TOOL] installing Python package via pipx"
pipx install --force "$SCRIPT_DIR"

# 4. Voice models (Piper EN + FR)
mkdir -p "$MODELS"
download_voice () {
    local voice="$1"
    local subdir="$2"
    local onnx_url="https://huggingface.co/rhasspy/piper-voices/resolve/main/${subdir}/${voice}.onnx"
    local cfg_url="https://huggingface.co/rhasspy/piper-voices/resolve/main/${subdir}/${voice}.onnx.json"
    if [[ -f "$MODELS/${voice}.onnx" && -f "$MODELS/${voice}.onnx.json" ]]; then
        echo "[$TOOL] $voice already present"
        return
    fi
    echo "[$TOOL] downloading $voice"
    curl -fSL --retry 3 -o "$MODELS/${voice}.onnx" "$onnx_url"
    curl -fSL --retry 3 -o "$MODELS/${voice}.onnx.json" "$cfg_url"
}
download_voice en_US-lessac-medium en/en_US/lessac/medium
download_voice fr_FR-siwis-medium  fr/fr_FR/siwis/medium

# 5. Systemd user unit
SYSTEMD="$HOME/.config/systemd/user"
mkdir -p "$SYSTEMD"
cp "$SCRIPT_DIR/systemd/math-speakd.service" "$SYSTEMD/"
systemctl --user daemon-reload
systemctl --user enable --now math-speakd.service || {
    echo "[$TOOL] systemctl enable failed (running headless?). Run manually:"
    echo "    math-speakd --foreground"
}

# 6. Desktop entry
APPS="$HOME/.local/share/applications"
mkdir -p "$APPS"
cat > "$APPS/math-speak.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Math Speak
Comment=Read selected mathematical text aloud (EN/FR)
Exec=math-speakd
Icon=accessories-text-editor
Categories=Utility;Accessibility;
Terminal=false
StartupNotify=false
EOF

echo "[$TOOL] install complete"
echo "[$TOOL] hotkey: Ctrl+Alt+M  (configurable in ~/.config/math-speak/config.toml)"
echo "[$TOOL] tray icon: click to switch EN ↔ FR"
echo "[$TOOL] log:  journalctl --user -u math-speakd -f"
