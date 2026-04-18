#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ASSETS="$ROOT/app/src/main/assets"
STAGE="$ROOT/.model-stage"

mkdir -p "$ASSETS/whisper" "$ASSETS/mt" "$ASSETS/tts" "$STAGE"

FAIL_COUNT=0

# ---------- Whisper ----------
WHISPER_OUT="$ASSETS/whisper/ggml-small-q5_1.bin"
if [[ ! -f "$WHISPER_OUT" ]]; then
  echo "[whisper] downloading ggml-small-q5_1.bin (~190 MB)"
  if ! curl -L --fail --progress-bar -o "$WHISPER_OUT" \
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small-q5_1.bin"; then
    echo "[whisper] FAILED"
    FAIL_COUNT=$((FAIL_COUNT + 1))
    rm -f "$WHISPER_OUT"
  else
    echo "[whisper] OK ($(du -h "$WHISPER_OUT" | cut -f1))"
  fi
else
  echo "[whisper] already present ($(du -h "$WHISPER_OUT" | cut -f1))"
fi

# ---------- OPUS-MT (6 pairs) ----------
# Requires python3 + pip; creates a local venv with ct2-transformers-converter.
PY_VENV="$STAGE/venv"
if [[ ! -d "$PY_VENV" ]]; then
  echo "[mt] creating Python venv at $PY_VENV"
  python3 -m venv "$PY_VENV"
fi

echo "[mt] installing conversion dependencies"
"$PY_VENV/bin/pip" install --upgrade pip --quiet
"$PY_VENV/bin/pip" install --quiet \
  "ctranslate2>=4.3,<5" \
  "transformers>=4.44,<5" \
  "sentencepiece>=0.2" \
  "torch>=2.2,<3" \
  "huggingface_hub>=0.24"

PAIRS=(en-hu hu-en fr-hu hu-fr en-fr fr-en)
for pair in "${PAIRS[@]}"; do
  SRC="${pair%-*}"; TGT="${pair#*-}"
  HF="Helsinki-NLP/opus-mt-${SRC}-${TGT}"
  OUT="$ASSETS/mt/opus-mt-${SRC}-${TGT}"
  if [[ -f "$OUT/model.bin" ]]; then
    echo "[mt] $pair: already converted"
    continue
  fi
  echo "[mt] $pair: converting from $HF"
  mkdir -p "$OUT"
  if ! "$PY_VENV/bin/ct2-transformers-converter" \
      --model "$HF" \
      --output_dir "$OUT" \
      --quantization int8 \
      --force \
      --copy_files source.spm target.spm tokenizer_config.json vocab.json 2>&1; then
    echo "[mt] $pair: ct2-transformers-converter FAILED; trying huggingface_hub fallback"
    if ! "$PY_VENV/bin/python3" -c "
from huggingface_hub import snapshot_download
import subprocess, sys, os
local = snapshot_download('$HF', local_dir='/tmp/opus-mt-${SRC}-${TGT}')
subprocess.run([sys.executable, '-m', 'ctranslate2.converters.opennmt_tf',
    '--help'], check=False)
# Re-run ct2 converter with local path
result = subprocess.run([
    '$PY_VENV/bin/ct2-transformers-converter',
    '--model', local,
    '--output_dir', '$OUT',
    '--quantization', 'int8',
    '--force',
    '--copy_files', 'source.spm', 'target.spm',
    'tokenizer_config.json', 'vocab.json'
], capture_output=False)
sys.exit(result.returncode)
"; then
      echo "[mt] $pair: FAILED (both paths)"
      FAIL_COUNT=$((FAIL_COUNT + 1))
      rm -rf "$OUT"
      continue
    fi
  fi
  # Helsinki repos ship source.spm and target.spm; if absent, fall back to spm.model
  [[ -f "$OUT/source.spm" ]] || cp "$OUT/spm.model" "$OUT/source.spm" 2>/dev/null || true
  [[ -f "$OUT/target.spm" ]] || cp "$OUT/spm.model" "$OUT/target.spm" 2>/dev/null || true
  echo "[mt] $pair: OK"
done

# ---------- Piper voices ----------
VOICES=(
  "en/en_US/lessac/medium/en_US-lessac-medium"
  "fr/fr_FR/siwis/medium/fr_FR-siwis-medium"
  "hu/hu_HU/anna/medium/hu_HU-anna-medium"
  # Fallback for anna: "hu/hu_HU/berta/medium/hu_HU-berta-medium"
)
for rel in "${VOICES[@]}"; do
  base="$(basename "$rel")"
  OUT="$ASSETS/tts/$base"
  mkdir -p "$OUT"
  for ext in onnx onnx.json; do
    if [[ ! -f "$OUT/model.${ext}" ]]; then
      echo "[tts] $base.$ext downloading"
      if ! curl -L --fail --progress-bar \
        -o "$OUT/model.${ext}" \
        "https://huggingface.co/rhasspy/piper-voices/resolve/main/${rel}.${ext}"; then
        echo "[tts] $base.$ext FAILED"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        rm -f "$OUT/model.${ext}"
      else
        echo "[tts] $base.$ext OK ($(du -h "$OUT/model.${ext}" | cut -f1))"
      fi
    else
      echo "[tts] $base.$ext already present"
    fi
  done
done

bash "$ROOT/scripts/verify-assets.sh" || true

if [[ $FAIL_COUNT -gt 0 ]]; then
  echo ""
  echo "WARNING: $FAIL_COUNT artifact(s) failed to download."
  echo "Re-run this script to retry failed downloads."
  exit 1
fi

echo ""
echo "All models ready under $ASSETS"
