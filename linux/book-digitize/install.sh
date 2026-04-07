#!/usr/bin/env bash
# install.sh — Install book-digitize and its dependencies
#
# Run once:
#   chmod +x install.sh && ./install.sh
#
# After that just run:  book-digitize input.mp4 --output book.txt
# (assuming ~/.local/bin is in your PATH)

set -euo pipefail

INSTALL_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.local/share/book-digitize"
VENV_DIR="$DATA_DIR/venv"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "════════════════════════════════════════"
echo " book-digitize installer"
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
command -v python3   >/dev/null 2>&1 || MISSING+=("python3")
command -v tesseract >/dev/null 2>&1 || MISSING+=("tesseract")

if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo "  Missing: ${MISSING[*]}"
    if [[ " ${MISSING[*]} " == *" tesseract "* ]]; then
        echo "  Install Tesseract with:"
        if [ "$PKG_MANAGER" = "apt" ]; then
            echo "    sudo apt-get install -y tesseract-ocr"
        elif [ "$PKG_MANAGER" = "pacman" ]; then
            echo "    sudo pacman -S tesseract"
        fi
    fi
    exit 1
fi

echo "  ✓ python3, tesseract found"

# ── 2. Tesseract language packs ─────────────────────────────────────────────
echo ""
echo "▶ Ensuring Tesseract language packs are installed…"

LANGS_NEEDED=()
INSTALLED_LANGS=$(tesseract --list-langs 2>&1 || true)

if ! echo "$INSTALLED_LANGS" | grep -q "^eng$"; then
    LANGS_NEEDED+=("eng")
fi
if ! echo "$INSTALLED_LANGS" | grep -q "^fra$"; then
    LANGS_NEEDED+=("fra")
fi

if [[ ${#LANGS_NEEDED[@]} -gt 0 ]]; then
    echo "  Installing language packs: ${LANGS_NEEDED[*]}"
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
echo "  Installing Python dependencies (includes Surya OCR + PyTorch — may take a few minutes)…"
"$VENV_DIR/bin/pip" install --quiet -r "$SCRIPT_DIR/requirements.txt"
echo "  ✓ Python dependencies installed"
echo ""
echo "  NOTE: Surya OCR will download ~1-2 GB of model weights on first run."
echo "  This is a one-time download cached in ~/.cache/huggingface/"

# ── 4. Install to ~/.local/bin ──────────────────────────────────────────────
echo ""
echo "▶ Installing to $INSTALL_DIR…"
mkdir -p "$INSTALL_DIR"

cp "$SCRIPT_DIR/book_digitize.py" "$INSTALL_DIR/book_digitize.py"
chmod +x "$INSTALL_DIR/book_digitize.py"

# Create wrapper script that activates the venv
cat > "$INSTALL_DIR/book-digitize" << WRAPPER
#!/usr/bin/env bash
# Auto-generated wrapper — runs book_digitize.py inside the book-digitize venv.
exec "$VENV_DIR/bin/python3" "$INSTALL_DIR/book_digitize.py" "\$@"
WRAPPER
chmod +x "$INSTALL_DIR/book-digitize"

echo "  ✓ book_digitize.py  → $INSTALL_DIR/book_digitize.py"
echo "  ✓ book-digitize     → $INSTALL_DIR/book-digitize (wrapper)"

# ── 5. PATH check ──────────────────────────────────────────────────────────
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

# ── 6. Done ─────────────────────────────────────────────────────────────────
echo ""
echo "════════════════════════════════════════"
echo " Installation complete!"
echo "════════════════════════════════════════"
echo ""
echo "Usage:"
echo "  book-digitize input.mp4 --output book.md --lang fra"
echo ""
echo "Options:"
echo "  -o, --output FILE         Output file (default: <input>.md)"
echo "  -f, --format {md,txt,pdf} Output format: md, txt, or pdf (default: md)"
echo "  -l, --lang LANG           Language: fr, en, fr+en (default: fr)"
echo "  --ocr-engine {surya,tesseract}  OCR engine (default: surya)"
echo "  --no-preprocess           Skip page detection/split/enhancement"
echo "  --frame-interval SECS     Frame extraction interval (default: 0.5)"
echo "  --sharpness-threshold N   Blur detection threshold (default: 50)"
echo "  --diff-threshold N        Page transition threshold (default: 30)"
echo "  --hash-threshold N        Page dedup threshold (default: 8)"
echo "  --page-crop-ratio N       Header/footer crop ratio (default: 0.08)"
echo "  --extract-images          Extract embedded images into images/ directory"
echo "  --claude-layout           Use Claude Vision for layout analysis (needs claude CLI)"
echo "  --pdf-margin CM           PDF margin in cm (default: 2.0)"
echo "  --pdf-font PATH           Path to TTF font for PDF (default: auto-detect)"
echo "  --max-claude-calls N      Max Claude API calls, 0=unlimited (default: 0)"
echo "  --keep-frames             Save page images to ./frames/"
echo "  --log FILE                Write summary log to file"
echo "  -v, --verbose             Detailed progress output"
