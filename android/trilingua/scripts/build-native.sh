#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Prereqs: ANDROID_NDK_HOME pointing at NDK r27+ installed via sdkmanager.
: "${ANDROID_NDK_HOME:?ANDROID_NDK_HOME must be set}"

OUT_DIR="$ROOT/app/src/main/jniLibs/arm64-v8a"
mkdir -p "$OUT_DIR"

# Gradle module tasks produce .so under their own build/intermediates dirs.
./gradlew :native-whisper:externalNativeBuildRelease :native-ct2:externalNativeBuildRelease

# Glob for the actual paths (release or debug variant)
WHISPER_SO=$(find "$ROOT/native-whisper/build/intermediates/cxx" -name "libwhisper_jni.so" 2>/dev/null | head -1)
CT2_JNI_SO=$(find "$ROOT/native-ct2/build/intermediates/cxx" -name "libct2_jni.so" 2>/dev/null | head -1)
CT2_SO=$(find "$ROOT/native-ct2/build/intermediates/cxx" -name "libctranslate2.so" 2>/dev/null | head -1)

if [[ -z "$WHISPER_SO" ]]; then
  echo "ERROR: libwhisper_jni.so not found in native-whisper/build"
  exit 1
fi
if [[ -z "$CT2_JNI_SO" ]]; then
  echo "ERROR: libct2_jni.so not found in native-ct2/build"
  exit 1
fi
if [[ -z "$CT2_SO" ]]; then
  echo "ERROR: libctranslate2.so not found in native-ct2/build"
  exit 1
fi

cp -v "$WHISPER_SO"  "$OUT_DIR/libwhisper_jni.so"
cp -v "$CT2_JNI_SO"  "$OUT_DIR/libct2_jni.so"
cp -v "$CT2_SO"      "$OUT_DIR/libctranslate2.so"

# libc++_shared.so from NDK sysroot.
LIBCXX="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/lib/aarch64-linux-android/libc++_shared.so"
if [[ -f "$LIBCXX" ]]; then
  cp -v "$LIBCXX" "$OUT_DIR/libc++_shared.so"
else
  echo "WARN: libc++_shared.so not found at $LIBCXX"
fi

# Verify 16KB page size compliance.
READELF="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-readelf"
if [[ -x "$READELF" ]]; then
  for f in "$OUT_DIR"/*.so; do
    if ! "$READELF" -l "$f" | grep -q "0x4000"; then
      echo "WARN: $f may not be 16KB-aligned"
    fi
  done
fi

echo ""
echo "Native libs built to $OUT_DIR"
ls -lh "$OUT_DIR"
