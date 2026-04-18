#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ASSETS="$ROOT/app/src/main/assets"
FAIL=0

check_file() {
  local path="$1"
  local min_bytes="$2"
  local label="$3"
  if [[ ! -f "$path" ]]; then
    echo "MISSING: $label -> $path"
    FAIL=$((FAIL + 1))
    return
  fi
  local size
  size=$(stat -c%s "$path" 2>/dev/null || stat -f%z "$path" 2>/dev/null)
  if [[ "$size" -lt "$min_bytes" ]]; then
    echo "TOO SMALL: $label ($size bytes, expected >= $min_bytes) -> $path"
    FAIL=$((FAIL + 1))
    return
  fi
  local sha
  sha=$(sha256sum "$path" 2>/dev/null | cut -d' ' -f1 || shasum -a 256 "$path" | cut -d' ' -f1)
  echo "OK: $label  $(du -h "$path" | cut -f1)  sha256=$sha"
}

echo "=== Whisper ==="
check_file "$ASSETS/whisper/ggml-small-q5_1.bin" $((150 * 1024 * 1024)) "ggml-small-q5_1.bin"

echo ""
echo "=== MT models ==="
PAIRS=(en-hu hu-en fr-hu hu-fr en-fr fr-en)
for pair in "${PAIRS[@]}"; do
  check_file "$ASSETS/mt/opus-mt-$pair/model.bin"   $((20 * 1024 * 1024)) "opus-mt-$pair/model.bin"
  check_file "$ASSETS/mt/opus-mt-$pair/config.json" 100                    "opus-mt-$pair/config.json"
  check_file "$ASSETS/mt/opus-mt-$pair/source.spm"  $((500 * 1024))        "opus-mt-$pair/source.spm"
  check_file "$ASSETS/mt/opus-mt-$pair/target.spm"  $((500 * 1024))        "opus-mt-$pair/target.spm"
  # Check vocabulary (shared_vocabulary.json or vocab.json)
  VOC_FILE=""
  for vf in shared_vocabulary.json vocab.json; do
    if [[ -f "$ASSETS/mt/opus-mt-$pair/$vf" ]]; then
      VOC_FILE="$vf"
      break
    fi
  done
  if [[ -n "$VOC_FILE" ]]; then
    check_file "$ASSETS/mt/opus-mt-$pair/$VOC_FILE" $((10 * 1024)) "opus-mt-$pair/$VOC_FILE"
  else
    echo "MISSING: opus-mt-$pair/*vocabulary.json (neither shared_vocabulary.json nor vocab.json found)"
    FAIL=$((FAIL + 1))
  fi
done

echo ""
echo "=== TTS voices ==="
VOICES=(en_US-lessac-medium fr_FR-siwis-medium hu_HU-anna-medium)
for voice in "${VOICES[@]}"; do
  check_file "$ASSETS/tts/$voice/model.onnx"      $((40 * 1024 * 1024)) "$voice/model.onnx"
  check_file "$ASSETS/tts/$voice/model.onnx.json" 100                    "$voice/model.onnx.json"
done

echo ""
if [[ $FAIL -gt 0 ]]; then
  echo "RESULT: $FAIL check(s) FAILED"
  exit 1
else
  echo "RESULT: all checks passed"
fi
