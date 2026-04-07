#!/usr/bin/env bash
# install.sh — Install gamebook-digitize and its dependencies
#
# Run once:
#   chmod +x install.sh && ./install.sh
#
# After that just run:  gamebook-digitize input.mp4 --lang fr --ref-pages 3
# (assuming ~/.local/bin is in your PATH)

set -euo pipefail

INSTALL_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.local/share/gamebook-digitize"
VENV_DIR="$DATA_DIR/venv"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "════════════════════════════════════════"
echo " gamebook-digitize installer"
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
command -v python3 >/dev/null 2>&1 || MISSING+=("python3")

if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo "  Missing: ${MISSING[*]}"
    exit 1
fi

echo "  ✓ python3 found"

# Check Python version (3.10+ required)
if ! python3 -c "import sys; sys.exit(0 if sys.version_info >= (3, 10) else 1)" 2>/dev/null; then
    PY_VER=$(python3 -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")
    echo "  ERROR: Python 3.10+ required (found $PY_VER)"
    exit 1
fi
echo "  ✓ Python version $(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")') OK"

# Check for tesseract (optional — only needed with --ocr-engine tesseract)
if command -v tesseract >/dev/null 2>&1; then
    echo "  ✓ tesseract found (optional)"

    # Check language packs
    LANGS_NEEDED=()
    INSTALLED_LANGS=$(tesseract --list-langs 2>&1 || true)

    if ! echo "$INSTALLED_LANGS" | grep -q "^eng$"; then
        LANGS_NEEDED+=("eng")
    fi
    if ! echo "$INSTALLED_LANGS" | grep -q "^fra$"; then
        LANGS_NEEDED+=("fra")
    fi

    if [[ ${#LANGS_NEEDED[@]} -gt 0 ]]; then
        echo "  Installing Tesseract language packs: ${LANGS_NEEDED[*]}"
        if [ "$PKG_MANAGER" = "apt" ]; then
            PKGS=()
            for lang in "${LANGS_NEEDED[@]}"; do
                PKGS+=("tesseract-ocr-$lang")
            done
            sudo apt-get install -y "${PKGS[@]}"
        elif [ "$PKG_MANAGER" = "pacman" ]; then
            PKGS=()
            for lang in "${LANGS_NEEDED[@]}"; do
                PKGS+=("tesseract-data-$lang")
            done
            sudo pacman -S --noconfirm "${PKGS[@]}"
        else
            echo "  ⚠ Please install Tesseract language packs manually:"
            for lang in "${LANGS_NEEDED[@]}"; do
                echo "    - $lang"
            done
        fi
    else
        echo "  ✓ eng and fra language packs already installed"
    fi
else
    echo "  ⚠ tesseract not found (optional — install if you want --ocr-engine tesseract)"
    if [ "$PKG_MANAGER" = "apt" ]; then
        echo "    sudo apt-get install -y tesseract-ocr tesseract-ocr-fra tesseract-ocr-eng"
    elif [ "$PKG_MANAGER" = "pacman" ]; then
        echo "    sudo pacman -S tesseract tesseract-data-fra tesseract-data-eng"
    fi
fi

# ── 2. Python virtual environment + dependencies ────────────────────────────
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
echo "  Installing Python dependencies (includes Surya OCR + PyTorch — may take a few minutes)…"
"$VENV_DIR/bin/pip" install --quiet -r "$SCRIPT_DIR/requirements.txt"
echo "  ✓ Python dependencies installed"
echo ""
echo "  NOTE: Surya OCR will download ~1-2 GB of model weights on first run."
echo "  This is a one-time download cached in ~/.cache/huggingface/"

# ── 3. Install source files + launcher ──────────────────────────────────────
echo ""
echo "▶ Installing to $DATA_DIR…"

cp "$SCRIPT_DIR/gamebook_digitize.py" "$DATA_DIR/gamebook_digitize.py"
cp "$SCRIPT_DIR/html_generator.py" "$DATA_DIR/html_generator.py"
echo "  ✓ Copied source files to $DATA_DIR/"

# Create wrapper script
mkdir -p "$INSTALL_DIR"
cat > "$INSTALL_DIR/gamebook-digitize" << WRAPPER
#!/usr/bin/env bash
# Auto-generated wrapper — runs gamebook_digitize.py inside the gamebook-digitize venv.
exec "$VENV_DIR/bin/python3" "$DATA_DIR/gamebook_digitize.py" "\$@"
WRAPPER
chmod +x "$INSTALL_DIR/gamebook-digitize"

echo "  ✓ gamebook-digitize → $INSTALL_DIR/gamebook-digitize (wrapper)"

# ── 4. PATH check ──────────────────────────────────────────────────────────
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

# ── 5. Done ─────────────────────────────────────────────────────────────────
echo ""
echo "════════════════════════════════════════"
echo " Installation complete!"
echo "════════════════════════════════════════"
echo ""
echo "Usage:"
echo "  gamebook-digitize input.mp4 --lang fr --ref-pages 3"
echo ""
echo "Options:"
echo "  -l, --lang {fr,en}          Book language (default: fr)"
echo "  --ref-pages N               Initial pages as reference images (default: 0)"
echo "  -o, --output DIR            Output directory (default: ./<input-stem>/)"
echo "  --ocr-engine {surya,tesseract}  OCR engine (default: surya)"
echo "  --no-llm                    Skip Claude CLI cleanup pass"
echo "  --from-markdown PATH        Generate HTML from existing markdown"
echo "  --frame-interval SECS       Frame extraction interval (default: 0.5)"
echo "  --sharpness-threshold N     Blur detection threshold (default: 50)"
echo "  --hash-threshold N          Page dedup threshold (default: 8)"
echo "  --keep-frames               Save selected page images to output/frames/"
echo "  --title TEXT                Book title (default: auto-detect)"
echo "  -v, --verbose               Detailed progress output"
