# Release Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish pre-built Linux binaries (x86_64 + aarch64) for each tool to GitHub Releases via a tag-triggered GitHub Actions workflow.

**Architecture:** A single `.github/workflows/release.yml` parses a tag like `voice-prompt-v1.0.0`, runs two parallel `cargo`/`cross` build jobs, then a third job assembles tarballs and creates the release. Each tool has an `install-release.sh` (no Rust toolchain required) that is renamed to `install.sh` in the tarball.

**Tech Stack:** GitHub Actions, cargo, `cross` (aarch64 cross-compilation), `gh` CLI, bash, HuggingFace CDN, github.com/tesseract-ocr/tessdata.

---

## File Map

| Action | Path |
|--------|------|
| Create | `.github/workflows/release.yml` |
| Create | `linux/voice-prompt/Cross.toml` |
| Create | `linux/voice-speak/Cross.toml` |
| Create | `linux/screen-ocr/Cross.toml` |
| Create | `linux/threshold-filter/Cross.toml` |
| Create | `linux/key-detect/Cross.toml` |
| Create | `steamdeck/deck-reader/Cross.toml` |
| Create | `linux/voice-prompt/install-release.sh` |
| Create | `linux/voice-speak/install-release.sh` |
| Create | `linux/screen-ocr/install-release.sh` |
| Create | `steamdeck/deck-reader/install-release.sh` |
| Create | `linux/threshold-filter/install-release.sh` |
| Create | `linux/key-detect/install-release.sh` |

---

## Task 1: Add Cross.toml to all 6 tool source dirs

`cross` needs a `Cross.toml` in the tool's workspace root to install ARM64 system headers (ALSA, X11, xcb, Wayland) into its Docker container. All 6 tools use the same content.

**Files:**
- Create: `linux/voice-prompt/Cross.toml`
- Create: `linux/voice-speak/Cross.toml`
- Create: `linux/screen-ocr/Cross.toml`
- Create: `linux/threshold-filter/Cross.toml`
- Create: `linux/key-detect/Cross.toml`
- Create: `steamdeck/deck-reader/Cross.toml`

- [ ] **Step 1: Create Cross.toml in each tool dir**

Write the following content to all 6 paths (identical):

```toml
[target.aarch64-unknown-linux-gnu]
pre-build = [
    "dpkg --add-architecture arm64",
    "apt-get update -qq",
    "apt-get install -y libx11-dev:arm64 libxkbcommon-dev:arm64 libwayland-dev:arm64 libxrandr-dev:arm64 libxi-dev:arm64 libxtst-dev:arm64 libxcb-render0-dev:arm64 libxcb-shape0-dev:arm64 libxcb-xfixes0-dev:arm64 libasound2-dev:arm64"
]
```

- [ ] **Step 2: Commit**

```bash
git add linux/voice-prompt/Cross.toml linux/voice-speak/Cross.toml \
        linux/screen-ocr/Cross.toml linux/threshold-filter/Cross.toml \
        linux/key-detect/Cross.toml steamdeck/deck-reader/Cross.toml
git commit -m "chore: add Cross.toml for aarch64 cross-compilation"
```

---

## Task 2: install-release.sh for key-detect

Simplest tool — no Python, no models, no config. Just binary copy.

**Files:**
- Create: `linux/key-detect/install-release.sh`

- [ ] **Step 1: Create the file**

```bash
#!/usr/bin/env bash
set -euo pipefail
TOOL="key-detect"
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  BIN="./${TOOL}-x86_64" ;;
  aarch64) BIN="./${TOOL}-aarch64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac
[ -f "$BIN" ] || { echo "Binary not found: $BIN" >&2; exit 1; }
mkdir -p ~/.local/bin
install -m 755 "$BIN" ~/.local/bin/$TOOL
echo "Done! Run: $TOOL"
echo "  Prints key codes to stdout. Press Ctrl+C to stop."
```

- [ ] **Step 2: Commit**

```bash
git add linux/key-detect/install-release.sh
git commit -m "feat(key-detect): add release installer"
```

---

## Task 3: install-release.sh for threshold-filter

Binary + config, no Python, no models.

**Files:**
- Create: `linux/threshold-filter/install-release.sh`

- [ ] **Step 1: Create the file**

```bash
#!/usr/bin/env bash
set -euo pipefail
TOOL="threshold-filter"
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  BIN="./${TOOL}-x86_64" ;;
  aarch64) BIN="./${TOOL}-aarch64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac
[ -f "$BIN" ] || { echo "Binary not found: $BIN" >&2; exit 1; }
mkdir -p ~/.local/bin
install -m 755 "$BIN" ~/.local/bin/$TOOL
mkdir -p ~/.config/$TOOL
if [ ! -f ~/.config/$TOOL/config.toml ]; then
    cp config.toml ~/.config/$TOOL/config.toml
    echo "  Config → ~/.config/$TOOL/config.toml (new)"
else
    echo "  Config → ~/.config/$TOOL/config.toml (existing, not overwritten)"
fi
echo "Done! Run: $TOOL"
```

- [ ] **Step 2: Commit**

```bash
git add linux/threshold-filter/install-release.sh
git commit -m "feat(threshold-filter): add release installer"
```

---

## Task 4: install-release.sh for voice-prompt

Binary + config + Python venv. Whisper model downloads automatically on first use via faster-whisper — no explicit download step.

**Files:**
- Create: `linux/voice-prompt/install-release.sh`

- [ ] **Step 1: Create the file**

```bash
#!/usr/bin/env bash
set -euo pipefail
TOOL="voice-prompt"
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  BIN="./${TOOL}-x86_64" ;;
  aarch64) BIN="./${TOOL}-aarch64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac
[ -f "$BIN" ] || { echo "Binary not found: $BIN" >&2; exit 1; }
echo "Installing $TOOL for $ARCH..."

# 1. Binary
mkdir -p ~/.local/bin
install -m 755 "$BIN" ~/.local/bin/$TOOL
echo "  Binary → ~/.local/bin/$TOOL"

# 2. Config (never overwrite existing)
mkdir -p ~/.config/$TOOL
if [ ! -f ~/.config/$TOOL/config.toml ]; then
    cp config.toml ~/.config/$TOOL/config.toml
    echo "  Config → ~/.config/$TOOL/config.toml (new)"
else
    echo "  Config → ~/.config/$TOOL/config.toml (existing, not overwritten)"
fi

# 3. Python script
mkdir -p ~/.local/share/$TOOL
cp python/whisper_transcribe.py ~/.local/share/$TOOL/
echo "  Script → ~/.local/share/$TOOL/whisper_transcribe.py"

# 4. Python venv + faster-whisper
VENV="$HOME/.local/share/$TOOL/venv"
if [ ! -d "$VENV" ]; then
    echo "  Creating Python venv..."
    python3 -m venv "$VENV"
fi
echo "  Installing faster-whisper..."
"$VENV/bin/pip" install --quiet faster-whisper
echo "  Python deps installed."

# 5. Wrapper script (binary expects this at ~/.local/bin/whisper_transcribe_wrapper.sh)
cat > ~/.local/bin/whisper_transcribe_wrapper.sh <<EOF
#!/usr/bin/env bash
exec "$VENV/bin/python3" "$HOME/.local/share/$TOOL/whisper_transcribe.py" "\$@"
EOF
chmod +x ~/.local/bin/whisper_transcribe_wrapper.sh
echo "  Wrapper → ~/.local/bin/whisper_transcribe_wrapper.sh"

# 6. Runtime dep check
MISSING=""
for dep in xdotool paplay; do
    command -v "$dep" >/dev/null 2>&1 || MISSING="$MISSING $dep"
done
[ -n "$MISSING" ] && echo "  Missing runtime deps:$MISSING — install: sudo apt install$MISSING"

# 7. Desktop entries
mkdir -p ~/.config/autostart
cat > ~/.config/autostart/${TOOL}.desktop <<DESKTOP
[Desktop Entry]
Type=Application
Name=voice-prompt
Exec=$HOME/.local/bin/$TOOL --daemon
Hidden=false
X-GNOME-Autostart-enabled=true
Comment=Push-to-talk speech transcription daemon
DESKTOP

mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/${TOOL}.desktop <<DESKTOP
[Desktop Entry]
Type=Application
Name=voice-prompt Settings
Exec=$HOME/.local/bin/$TOOL
Icon=audio-input-microphone
Comment=Configure voice-prompt settings
Categories=Utility;Accessibility;
DESKTOP

echo ""
echo "Done! $TOOL installed."
echo "  Run daemon:    $TOOL --daemon"
echo "  Open settings: $TOOL"
echo "  Note: Whisper 'small' model (~244 MB) downloads on first use."
```

- [ ] **Step 2: Commit**

```bash
git add linux/voice-prompt/install-release.sh
git commit -m "feat(voice-prompt): add release installer"
```

---

## Task 5: install-release.sh for voice-speak

Binary + config + Python venv + Piper model download (~122 MB for en+fr).

**Files:**
- Create: `linux/voice-speak/install-release.sh`

- [ ] **Step 1: Create the file**

```bash
#!/usr/bin/env bash
set -euo pipefail
TOOL="voice-speak"
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  BIN="./${TOOL}-x86_64" ;;
  aarch64) BIN="./${TOOL}-aarch64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac
[ -f "$BIN" ] || { echo "Binary not found: $BIN" >&2; exit 1; }
echo "Installing $TOOL for $ARCH..."

# 1. Binary
mkdir -p ~/.local/bin
install -m 755 "$BIN" ~/.local/bin/$TOOL
echo "  Binary → ~/.local/bin/$TOOL"

# 2. Config (never overwrite existing)
mkdir -p ~/.config/$TOOL
if [ ! -f ~/.config/$TOOL/config.toml ]; then
    cp config.toml ~/.config/$TOOL/config.toml
    echo "  Config → ~/.config/$TOOL/config.toml (new)"
else
    echo "  Config → ~/.config/$TOOL/config.toml (existing, not overwritten)"
fi

# 3. Python script
mkdir -p ~/.local/share/$TOOL
cp python/tts_speak.py ~/.local/share/$TOOL/
echo "  Script → ~/.local/share/$TOOL/tts_speak.py"

# 4. Python venv + piper-tts
VENV="$HOME/.local/share/$TOOL/venv"
if [ ! -d "$VENV" ]; then
    echo "  Creating Python venv..."
    python3 -m venv "$VENV"
fi
echo "  Installing piper-tts..."
"$VENV/bin/pip" install --quiet piper-tts sounddevice numpy
echo "  Python deps installed."

# 5. Wrapper script
cat > ~/.local/bin/tts_speak_wrapper.sh <<EOF
#!/usr/bin/env bash
exec "$VENV/bin/python3" "$HOME/.local/share/$TOOL/tts_speak.py" "\$@"
EOF
chmod +x ~/.local/bin/tts_speak_wrapper.sh
echo "  Wrapper → ~/.local/bin/tts_speak_wrapper.sh"

# 6. Download Piper models
MODEL_DIR="$HOME/.local/share/$TOOL/models"
mkdir -p "$MODEL_DIR"
PIPER_BASE="https://huggingface.co/rhasspy/piper-voices/resolve/main"

download_piper_model() {
    local name="$1"
    local url_path="$2"
    if [ ! -f "$MODEL_DIR/${name}.onnx" ]; then
        echo "  Downloading Piper model: $name (~61 MB)..."
        curl -L --fail --progress-bar \
            -o "$MODEL_DIR/${name}.onnx" \
            "$PIPER_BASE/${url_path}/${name}.onnx"
        curl -L --fail -s \
            -o "$MODEL_DIR/${name}.onnx.json" \
            "$PIPER_BASE/${url_path}/${name}.onnx.json"
        echo "  $name installed."
    else
        echo "  $name already present, skipping."
    fi
}

download_piper_model "en_US-lessac-medium" "en/en_US/lessac/medium"
download_piper_model "fr_FR-siwis-medium"  "fr/fr_FR/siwis/medium"

# 7. Runtime dep check
MISSING=""
for dep in paplay; do
    command -v "$dep" >/dev/null 2>&1 || MISSING="$MISSING $dep"
done
[ -n "$MISSING" ] && echo "  Missing runtime deps:$MISSING — install: sudo apt install$MISSING"

# 8. Desktop entries
mkdir -p ~/.config/autostart
cat > ~/.config/autostart/${TOOL}.desktop <<DESKTOP
[Desktop Entry]
Type=Application
Name=voice-speak
Exec=$HOME/.local/bin/$TOOL --daemon
Hidden=false
X-GNOME-Autostart-enabled=true
Comment=Text-to-speech daemon
DESKTOP

mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/${TOOL}.desktop <<DESKTOP
[Desktop Entry]
Type=Application
Name=voice-speak Settings
Exec=$HOME/.local/bin/$TOOL
Icon=audio-output-speaker
Comment=Configure voice-speak settings
Categories=Utility;Accessibility;
DESKTOP

echo ""
echo "Done! $TOOL installed."
echo "  Run daemon:    $TOOL --daemon"
echo "  Open settings: $TOOL"
```

- [ ] **Step 2: Commit**

```bash
git add linux/voice-speak/install-release.sh
git commit -m "feat(voice-speak): add release installer"
```

---

## Task 6: install-release.sh for screen-ocr

Binary + config + Python venv + tessdata download + Piper model download. The wrapper sets `TESSDATA_PREFIX` so pytesseract uses the downloaded data instead of the system default.

**Files:**
- Create: `linux/screen-ocr/install-release.sh`

- [ ] **Step 1: Create the file**

```bash
#!/usr/bin/env bash
set -euo pipefail
TOOL="screen-ocr"
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  BIN="./${TOOL}-x86_64" ;;
  aarch64) BIN="./${TOOL}-aarch64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac
[ -f "$BIN" ] || { echo "Binary not found: $BIN" >&2; exit 1; }
echo "Installing $TOOL for $ARCH..."

# 1. Binary
mkdir -p ~/.local/bin
install -m 755 "$BIN" ~/.local/bin/$TOOL
echo "  Binary → ~/.local/bin/$TOOL"

# 2. Config (never overwrite existing)
mkdir -p ~/.config/$TOOL
if [ ! -f ~/.config/$TOOL/config.toml ]; then
    cp config.toml ~/.config/$TOOL/config.toml
    echo "  Config → ~/.config/$TOOL/config.toml (new)"
else
    echo "  Config → ~/.config/$TOOL/config.toml (existing, not overwritten)"
fi

# 3. Python script
mkdir -p ~/.local/share/$TOOL
cp python/ocr_extract.py ~/.local/share/$TOOL/
echo "  Script → ~/.local/share/$TOOL/ocr_extract.py"

# 4. Python venv + pytesseract
VENV="$HOME/.local/share/$TOOL/venv"
if [ ! -d "$VENV" ]; then
    echo "  Creating Python venv..."
    python3 -m venv "$VENV"
fi
echo "  Installing Python deps..."
"$VENV/bin/pip" install --quiet -r python/requirements.txt
echo "  Python deps installed."

# 5. Download tessdata
TESS_DIR="$HOME/.local/share/$TOOL/models/tessdata"
mkdir -p "$TESS_DIR"
TESS_BASE="https://github.com/tesseract-ocr/tessdata/raw/main"
for lang_size in "eng:4 MB" "fra:1 MB"; do
    lang="${lang_size%%:*}"
    size="${lang_size##*:}"
    if [ ! -f "$TESS_DIR/${lang}.traineddata" ]; then
        echo "  Downloading tessdata: $lang (~$size)..."
        curl -L --fail --progress-bar \
            -o "$TESS_DIR/${lang}.traineddata" \
            "$TESS_BASE/${lang}.traineddata"
    else
        echo "  tessdata/$lang already present, skipping."
    fi
done

# 6. Download Piper TTS model (en only; screen-ocr reads aloud in English)
PIPER_DIR="$HOME/.local/share/voice-speak/models"
mkdir -p "$PIPER_DIR"
PIPER_BASE="https://huggingface.co/rhasspy/piper-voices/resolve/main"
if [ ! -f "$PIPER_DIR/en_US-lessac-medium.onnx" ]; then
    echo "  Downloading Piper TTS model: en_US-lessac-medium (~61 MB)..."
    curl -L --fail --progress-bar \
        -o "$PIPER_DIR/en_US-lessac-medium.onnx" \
        "$PIPER_BASE/en/en_US/lessac/medium/en_US-lessac-medium.onnx"
    curl -L --fail -s \
        -o "$PIPER_DIR/en_US-lessac-medium.onnx.json" \
        "$PIPER_BASE/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json"
else
    echo "  Piper en_US-lessac-medium already present, skipping."
fi

# 7. Wrapper (sets TESSDATA_PREFIX so pytesseract uses downloaded tessdata)
cat > ~/.local/bin/ocr_extract_wrapper.sh <<EOF
#!/usr/bin/env bash
export TESSDATA_PREFIX="$HOME/.local/share/$TOOL/models"
exec "$VENV/bin/python3" "$HOME/.local/share/$TOOL/ocr_extract.py" "\$@"
EOF
chmod +x ~/.local/bin/ocr_extract_wrapper.sh
echo "  Wrapper → ~/.local/bin/ocr_extract_wrapper.sh"

# 8. Runtime dep check
MISSING=""
for dep in tesseract maim slop xclip paplay; do
    command -v "$dep" >/dev/null 2>&1 || MISSING="$MISSING $dep"
done
[ -n "$MISSING" ] && echo "  Missing runtime deps:$MISSING — install: sudo apt install$MISSING"

# 9. Desktop entries
mkdir -p ~/.config/autostart
cat > ~/.config/autostart/${TOOL}.desktop <<DESKTOP
[Desktop Entry]
Type=Application
Name=screen-ocr
Exec=$HOME/.local/bin/$TOOL --daemon
Hidden=false
X-GNOME-Autostart-enabled=true
Comment=Screen OCR daemon
DESKTOP

mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/${TOOL}.desktop <<DESKTOP
[Desktop Entry]
Type=Application
Name=screen-ocr Settings
Exec=$HOME/.local/bin/$TOOL
Icon=scanner
Comment=Configure screen-ocr settings
Categories=Utility;Accessibility;
DESKTOP

echo ""
echo "Done! $TOOL installed."
echo "  Run daemon:    $TOOL --daemon"
echo "  Open settings: $TOOL"
```

- [ ] **Step 2: Commit**

```bash
git add linux/screen-ocr/install-release.sh
git commit -m "feat(screen-ocr): add release installer"
```

---

## Task 7: install-release.sh for deck-reader

deck-reader combines OCR + TTS, needs tessdata + Piper (en only). Lives under `steamdeck/deck-reader/`.

**Files:**
- Create: `steamdeck/deck-reader/install-release.sh`

- [ ] **Step 1: Create the file**

```bash
#!/usr/bin/env bash
set -euo pipefail
TOOL="deck-reader"
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  BIN="./${TOOL}-x86_64" ;;
  aarch64) BIN="./${TOOL}-aarch64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac
[ -f "$BIN" ] || { echo "Binary not found: $BIN" >&2; exit 1; }
echo "Installing $TOOL for $ARCH..."

# 1. Binary
mkdir -p ~/.local/bin
install -m 755 "$BIN" ~/.local/bin/$TOOL
echo "  Binary → ~/.local/bin/$TOOL"

# 2. Config (never overwrite existing)
mkdir -p ~/.config/$TOOL
if [ ! -f ~/.config/$TOOL/config.toml ]; then
    cp config.toml ~/.config/$TOOL/config.toml
    echo "  Config → ~/.config/$TOOL/config.toml (new)"
else
    echo "  Config → ~/.config/$TOOL/config.toml (existing, not overwritten)"
fi

# 3. Python scripts
mkdir -p ~/.local/share/$TOOL
cp python/*.py ~/.local/share/$TOOL/
echo "  Scripts → ~/.local/share/$TOOL/"

# 4. Python venv + deps
VENV="$HOME/.local/share/$TOOL/venv"
if [ ! -d "$VENV" ]; then
    echo "  Creating Python venv..."
    python3 -m venv "$VENV"
fi
echo "  Installing Python deps..."
"$VENV/bin/pip" install --quiet -r python/requirements.txt
echo "  Python deps installed."

# 5. Download tessdata
TESS_DIR="$HOME/.local/share/$TOOL/models/tessdata"
mkdir -p "$TESS_DIR"
TESS_BASE="https://github.com/tesseract-ocr/tessdata/raw/main"
for lang_size in "eng:4 MB" "fra:1 MB"; do
    lang="${lang_size%%:*}"
    size="${lang_size##*:}"
    if [ ! -f "$TESS_DIR/${lang}.traineddata" ]; then
        echo "  Downloading tessdata: $lang (~$size)..."
        curl -L --fail --progress-bar \
            -o "$TESS_DIR/${lang}.traineddata" \
            "$TESS_BASE/${lang}.traineddata"
    else
        echo "  tessdata/$lang already present, skipping."
    fi
done

# 6. Download Piper model (en only)
PIPER_DIR="$HOME/.local/share/voice-speak/models"
mkdir -p "$PIPER_DIR"
PIPER_BASE="https://huggingface.co/rhasspy/piper-voices/resolve/main"
if [ ! -f "$PIPER_DIR/en_US-lessac-medium.onnx" ]; then
    echo "  Downloading Piper TTS model: en_US-lessac-medium (~61 MB)..."
    curl -L --fail --progress-bar \
        -o "$PIPER_DIR/en_US-lessac-medium.onnx" \
        "$PIPER_BASE/en/en_US/lessac/medium/en_US-lessac-medium.onnx"
    curl -L --fail -s \
        -o "$PIPER_DIR/en_US-lessac-medium.onnx.json" \
        "$PIPER_BASE/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json"
else
    echo "  Piper en_US-lessac-medium already present, skipping."
fi

# 7. OCR wrapper (sets TESSDATA_PREFIX)
cat > ~/.local/bin/ocr_extract_wrapper.sh <<EOF
#!/usr/bin/env bash
export TESSDATA_PREFIX="$HOME/.local/share/$TOOL/models"
exec "$VENV/bin/python3" "$HOME/.local/share/$TOOL/ocr_extract.py" "\$@"
EOF
chmod +x ~/.local/bin/ocr_extract_wrapper.sh

# TTS wrapper
cat > ~/.local/bin/tts_speak_wrapper.sh <<EOF
#!/usr/bin/env bash
exec "$VENV/bin/python3" "$HOME/.local/share/$TOOL/tts_speak.py" "\$@"
EOF
chmod +x ~/.local/bin/tts_speak_wrapper.sh

echo "  Wrappers → ~/.local/bin/ocr_extract_wrapper.sh, tts_speak_wrapper.sh"

# 8. Runtime dep check
MISSING=""
for dep in tesseract paplay; do
    command -v "$dep" >/dev/null 2>&1 || MISSING="$MISSING $dep"
done
[ -n "$MISSING" ] && echo "  Missing runtime deps:$MISSING — install: sudo apt install$MISSING"

echo ""
echo "Done! $TOOL installed."
echo "  Run daemon:    $TOOL --daemon"
echo "  Open settings: $TOOL"
```

- [ ] **Step 2: Commit**

```bash
git add steamdeck/deck-reader/install-release.sh
git commit -m "feat(deck-reader): add release installer"
```

---

## Task 8: Write .github/workflows/release.yml

The single workflow that handles all 6 tools. Uses `taiki-e/install-action` to install `cross` quickly from prebuilt binaries.

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Create the workflow file**

```yaml
name: Release

on:
  push:
    tags: ['*-v*.*.*']

permissions:
  contents: write

jobs:
  build:
    name: Build ${{ matrix.arch }}
    runs-on: ubuntu-latest
    strategy:
      matrix:
        include:
          - arch: x86_64
            target: x86_64-unknown-linux-gnu
            use_cross: false
          - arch: aarch64
            target: aarch64-unknown-linux-gnu
            use_cross: true

    steps:
      - uses: actions/checkout@v4

      - name: Parse tag
        id: parse
        run: |
          TAG="${GITHUB_REF_NAME}"
          VERSION="${TAG##*-v}"
          TOOL="${TAG%-v*}"
          case "$TOOL" in
            deck-reader) SRC_DIR="steamdeck/deck-reader" ;;
            *)           SRC_DIR="linux/$TOOL" ;;
          esac
          echo "tool=$TOOL"       >> "$GITHUB_OUTPUT"
          echo "version=$VERSION" >> "$GITHUB_OUTPUT"
          echo "src_dir=$SRC_DIR" >> "$GITHUB_OUTPUT"

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            ${{ steps.parse.outputs.src_dir }}/target
          key: ${{ steps.parse.outputs.tool }}-${{ matrix.target }}-${{ hashFiles(format('{0}/Cargo.lock', steps.parse.outputs.src_dir)) }}
          restore-keys: ${{ steps.parse.outputs.tool }}-${{ matrix.target }}-

      - name: Install cross
        if: matrix.use_cross
        uses: taiki-e/install-action@v2
        with:
          tool: cross

      - name: Build
        working-directory: ${{ steps.parse.outputs.src_dir }}
        run: |
          if [ "${{ matrix.use_cross }}" = "true" ]; then
            cross build --release --target ${{ matrix.target }}
          else
            cargo build --release --target ${{ matrix.target }}
          fi

      - name: Upload binary
        uses: actions/upload-artifact@v4
        with:
          name: binary-${{ matrix.arch }}
          path: ${{ steps.parse.outputs.src_dir }}/target/${{ matrix.target }}/release/${{ steps.parse.outputs.tool }}

  create-release:
    needs: build
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Parse tag
        id: parse
        run: |
          TAG="${GITHUB_REF_NAME}"
          VERSION="${TAG##*-v}"
          TOOL="${TAG%-v*}"
          case "$TOOL" in
            deck-reader) SRC_DIR="steamdeck/deck-reader" ;;
            *)           SRC_DIR="linux/$TOOL" ;;
          esac
          echo "tool=$TOOL"       >> "$GITHUB_OUTPUT"
          echo "version=$VERSION" >> "$GITHUB_OUTPUT"
          echo "src_dir=$SRC_DIR" >> "$GITHUB_OUTPUT"

      - name: Download binaries
        uses: actions/download-artifact@v4
        with:
          path: artifacts/

      - name: Assemble tarballs
        run: |
          TOOL="${{ steps.parse.outputs.tool }}"
          VERSION="${{ steps.parse.outputs.version }}"
          SRC="${{ steps.parse.outputs.src_dir }}"

          for ARCH in x86_64 aarch64; do
            DIR="${TOOL}-${VERSION}"
            mkdir -p "$DIR"

            # Binary (named <tool>-<arch> in tarball)
            cp "artifacts/binary-${ARCH}/${TOOL}" "${DIR}/${TOOL}-${ARCH}"
            chmod +x "${DIR}/${TOOL}-${ARCH}"

            # install-release.sh → install.sh
            cp "${SRC}/install-release.sh" "${DIR}/install.sh"
            chmod +x "${DIR}/install.sh"

            # config.toml (if present)
            [ -f "${SRC}/config.toml" ] && cp "${SRC}/config.toml" "${DIR}/config.toml"

            # python/ dir: copy python/ subdir, or tts_speak.py at root (voice-speak)
            if [ -d "${SRC}/python" ]; then
              cp -r "${SRC}/python" "${DIR}/python"
            elif [ -f "${SRC}/tts_speak.py" ]; then
              mkdir -p "${DIR}/python"
              cp "${SRC}/tts_speak.py" "${DIR}/python/"
              [ -f "${SRC}/requirements.txt" ] && cp "${SRC}/requirements.txt" "${DIR}/python/"
            fi

            # README
            cp "${SRC}/README.md" "${DIR}/README.md"

            tar czf "${TOOL}-${VERSION}-${ARCH}-linux.tar.gz" "${DIR}/"
            rm -rf "${DIR}"
          done

      - name: Build release notes
        run: |
          TOOL="${{ steps.parse.outputs.tool }}"
          VERSION="${{ steps.parse.outputs.version }}"
          SRC="${{ steps.parse.outputs.src_dir }}"

          # First non-heading paragraph from README
          DESC=$(awk '/^## /{exit} /^# [^#]/{found=1; next} found && /^[^[:space:]]/{print; exit}' "${SRC}/README.md")

          cat > release_body.md <<NOTES
          ## ${TOOL} v${VERSION}

          ${DESC}

          ## Install

          \`\`\`bash
          ARCH=\$(uname -m)
          curl -LO https://github.com/Gofer59/tools/releases/download/${TOOL}-v${VERSION}/${TOOL}-${VERSION}-\${ARCH}-linux.tar.gz
          tar xzf ${TOOL}-${VERSION}-\${ARCH}-linux.tar.gz
          cd ${TOOL}-${VERSION}
          bash install.sh
          \`\`\`
          NOTES

      - name: Create GitHub Release
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          TOOL="${{ steps.parse.outputs.tool }}"
          VERSION="${{ steps.parse.outputs.version }}"
          gh release create "${GITHUB_REF_NAME}" \
            --title "${TOOL} v${VERSION}" \
            --notes-file release_body.md \
            "${TOOL}-${VERSION}-x86_64-linux.tar.gz" \
            "${TOOL}-${VERSION}-aarch64-linux.tar.gz"
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "feat: add GitHub Actions release workflow"
```

---

## Task 9: Test with key-detect tag

key-detect is the simplest tool (no Python, no models). Test the full pipeline with it first before tagging the complex tools.

- [ ] **Step 1: Push all commits to origin**

```bash
git push
```

Expected: push succeeds, no workflow triggered (no tag yet).

- [ ] **Step 2: Push a key-detect test tag**

```bash
git tag key-detect-v1.0.0
git push origin key-detect-v1.0.0
```

Expected: GitHub Actions workflow triggers at `github.com/Gofer59/tools/actions`.

- [ ] **Step 3: Monitor the workflow run**

Go to `https://github.com/Gofer59/tools/actions` and watch the run for `key-detect-v1.0.0`.

Expected:
- `Build x86_64` job: PASS (~2 min)
- `Build aarch64` job: PASS (~5 min, cross pulls Docker image first run)
- `create-release` job: PASS (~1 min)

- [ ] **Step 4: Verify the release**

Go to `https://github.com/Gofer59/tools/releases/tag/key-detect-v1.0.0`.

Expected:
- Title: `key-detect v1.0.0`
- Two assets: `key-detect-1.0.0-x86_64-linux.tar.gz`, `key-detect-1.0.0-aarch64-linux.tar.gz`
- Release body contains install snippet with correct URLs

- [ ] **Step 5: Download and verify the tarball locally**

```bash
cd /tmp
curl -LO https://github.com/Gofer59/tools/releases/download/key-detect-v1.0.0/key-detect-1.0.0-x86_64-linux.tar.gz
tar tzf key-detect-1.0.0-x86_64-linux.tar.gz
```

Expected output:
```
key-detect-1.0.0/
key-detect-1.0.0/key-detect-x86_64
key-detect-1.0.0/install.sh
key-detect-1.0.0/README.md
```

- [ ] **Step 6: If workflow fails, check the Actions log**

Common failures and fixes:
- `Binary not found in artifact` → check that `steps.parse.outputs.tool` matches the Cargo package `name` in `Cargo.toml`
- `cross: command not found` → verify `taiki-e/install-action@v2` step ran; check Actions log for install errors
- `gh release create: already exists` → delete the release and tag on GitHub, re-tag locally: `git tag -d key-detect-v1.0.0 && git push origin :key-detect-v1.0.0`, then redo Step 2

- [ ] **Step 7: After key-detect succeeds, tag remaining tools**

```bash
git tag voice-prompt-v1.0.0
git tag voice-speak-v1.0.0
git tag screen-ocr-v1.0.0
git tag deck-reader-v1.0.0
git tag threshold-filter-v1.0.0
git push origin --tags
```

Each tag triggers an independent workflow run.
