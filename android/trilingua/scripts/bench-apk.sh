#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APK="$ROOT/app/build/outputs/apk/debug/app-debug.apk"

if [[ ! -f "$APK" ]]; then
  echo "APK not found at $APK"
  echo "Run: cd $ROOT && ./gradlew assembleDebug"
  exit 1
fi

echo "=== APK size ==="
ls -lh "$APK"
echo ""

# apkanalyzer
APKANALYZER="$ANDROID_HOME/cmdline-tools/latest/bin/apkanalyzer"
if [[ -x "$APKANALYZER" ]]; then
  echo "=== Component sizes ==="
  "$APKANALYZER" apk summary "$APK"
  echo ""
  echo "=== File sizes ==="
  "$APKANALYZER" files list "$APK" | sort -k2 -rn | head -30
else
  echo "apkanalyzer not found at $APKANALYZER; skipping detailed analysis"
fi

# zipalign check
ZIPALIGN="$ANDROID_HOME/build-tools/34.0.0/zipalign"
if [[ -x "$ZIPALIGN" ]]; then
  echo ""
  echo "=== zipalign verification ==="
  if "$ZIPALIGN" -c 4 "$APK"; then
    echo "APK is properly zipaligned"
  else
    echo "WARN: APK is NOT zipaligned"
  fi
fi
