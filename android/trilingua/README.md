# Trilingua

Fully offline Android speech translator for English, French, and Hungarian — all 6 directed pairs.

## Features

- Push-to-hold mic button
- All 3 languages + 6 translation pairs with no pivoting
- Zero network at runtime (NO internet permission)
- ~881 MB APK (all models embedded)

## Supported pairs

| # | Direction | STT | MT model | TTS voice |
|---|-----------|-----|----------|-----------|
| 1 | EN → HU | whisper small q5_1 | opus-mt-en-hu int8 | hu_HU-anna-medium |
| 2 | HU → EN | whisper small q5_1 | opus-mt-hu-en int8 | en_US-lessac-medium |
| 3 | FR → HU | whisper small q5_1 | opus-mt-fr-hu int8 | hu_HU-anna-medium |
| 4 | HU → FR | whisper small q5_1 | opus-mt-hu-fr int8 | fr_FR-siwis-medium |
| 5 | EN → FR | whisper small q5_1 | opus-mt-en-fr int8 | fr_FR-siwis-medium |
| 6 | FR → EN | whisper small q5_1 | opus-mt-fr-en int8 | en_US-lessac-medium |

## Install

1. Enable "Install from unknown sources" on your Android device (Settings → Security)
2. `adb install -r app/build/outputs/apk/debug/app-debug.apk`
3. Grant microphone permission on first launch
4. Wait for the BootProgressOverlay (~30 s on first launch) to complete model extraction

## Permissions

- `RECORD_AUDIO` — required for hold-to-talk microphone
- No internet permission — all inference is fully offline

## APK size breakdown

| Category | Size |
|----------|------|
| STT (whisper ggml-small-q5_1) | ~190 MB |
| MT (6 × opus-mt int8) | ~360 MB |
| TTS (3 × piper voices) | ~180 MB |
| Code + native libs | ~20 MB |
| **Total** | **~881 MB** |

On first launch, models are extracted to `~881 MB` in `filesDir` (internal storage). Subsequent launches skip extraction.

## Build from source

### Prerequisites

- JDK 21 (full JDK with jlink, e.g. `~/jdk-21.0.2`)
- Android SDK Platform 34 + Build-Tools 34.0.0
- Android NDK r27.1.12297006
- Android CMake 3.22.1
- Python 3.11+ (for model conversion)
- `curl`, `git`

### Steps

```bash
cd tools/trilingua

# 1. Download and convert models (~881 MB, ~15 min)
bash scripts/download-models.sh

# 2. Build native JNI libraries (requires NDK)
export ANDROID_NDK_HOME=/path/to/Android/Sdk/ndk/27.1.12297006
bash scripts/build-native.sh

# 3. Assemble APK
./gradlew assembleDebug
# APK at: app/build/outputs/apk/debug/app-debug.apk

# 4. Install to device
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

## Architecture

```
Android app (Kotlin + Jetpack Compose)
├── STT: whisper.cpp (JNI) — multilingual small model, greedy decoding
├── MT:  CTranslate2 (JNI) + SentencePiece — 6 direct-pair OPUS-MT int8 models
└── TTS: sherpa-onnx OfflineTts — Piper VITS voices for EN/FR/HU
```

Audio pipeline: `AudioRecord (16kHz PCM16) → whisper_jni → ct2_jni → sherpa-onnx → AudioTrack`

## Known Limitations

- **Hungarian STT quality**: whisper small q5_1 accuracy on Hungarian is unmeasured against Common Voice 17. Upgrade to `ggml-medium-q5_0.bin` (single-file swap in `scripts/download-models.sh`) if quality is insufficient.
- **Piper hu_HU-anna-medium mouth-click artifacts**: Known upstream Piper regression reported on GitHub. Replace with `hu_HU-berta-medium` in `scripts/download-models.sh` and `PiperTextToSpeechEngine.kt` if audible on device.
- **APK size ~881 MB**: Not suitable for Play Store upload without AAB + Play Asset Delivery refactor (each language pack as an install-time asset pack). Sideload via `adb install` only for v1. Debug keystore only — no release signing configured.
- **CTranslate2 Android build fragility**: CT2 on Android with NDK r27 requires careful CMake configuration. Build currently uses OpenMP disabled and RUY backend enabled.
- **First-boot extraction**: ~30 s to copy models from APK assets to internal storage on first launch.
- **arm64-v8a only**: No x86_64 or armeabi-v7a ABI support.

## Test matrix

14 manual test cases:

| # | Case | Steps | Pass criteria |
|---|------|-------|---------------|
| 1 | EN→HU | Set EN→HU, hold mic ~3 s, say "Good morning, how are you?" | Source pane shows EN transcript; target pane shows Hungarian; HU audio plays |
| 2 | HU→EN | Set HU→EN, hold mic ~3 s, say "Jó reggelt, hogy vagy?" | Source HU transcript; EN translation; EN audio |
| 3 | FR→HU | Set FR→HU, hold mic ~3 s, say "Bonjour, comment allez-vous ?" | FR source; HU target; HU audio |
| 4 | HU→FR | Set HU→FR, hold mic ~3 s, Hungarian greeting | HU source; FR target; FR audio |
| 5 | EN→FR | Set EN→FR, hold mic ~3 s | EN source; FR target; FR audio |
| 6 | FR→EN | Set FR→EN, hold mic ~3 s | FR source; EN target; EN audio |
| 7 | Swap round-trip | Finish a translation; tap swap; verify dropdowns reflect new direction | Dropdowns and panes flip; next press starts recording with new direction |
| 8 | Mic permission denied | Fresh install; deny permission at prompt | ErrorBanner `MicDenied`; mic button stays disabled |
| 9 | Rapid language change during recording | Press mic; while recording, try to change dropdown | Dropdown disabled while recording (`LanguageRow enabled=false`) |
| 10 | Very short utterance | Tap mic < 300 ms | ErrorBanner `TooShort`; no translation attempted |
| 11 | Very long utterance | Hold mic > 60 s | Auto-stops at 60 s; ErrorBanner `TooLong` (informational); translation runs on captured audio |
| 12 | Airplane mode | Enable airplane mode; run tests 1 and 4; run `adb shell dumpsys netstats detail` | No new network connections from `com.trilingua.app` UID |
| 13 | Fresh-install first boot | `adb uninstall com.trilingua.app`; install; launch | BootProgressOverlay visible with % during extraction; ready within ~30 s |
| 14 | Cold launch time | Kill app; `adb shell am force-stop com.trilingua.app`; measure launch to interactive | ≤ 4 s to mic button enabled (assuming extraction already done) |

Network isolation check (test 12):

```bash
adb shell dumpsys netstats detail | grep -A2 "uid=$(adb shell pm list packages -U com.trilingua.app | awk -F: '{print $3}')"
```

Expected: `rxBytes=0 txBytes=0` after a session of translations.

## UI Icon contentDescription conventions

- **Icon inside a Button with a visible Text label**: `contentDescription = null` (decorative — the label conveys the action).
- **Standalone Icon or IconButton without a text label**: `contentDescription = stringResource(R.string.…)` naming the action.
- **IconButton where icon and state both vary**: description must reflect the current enabled/pressed state, not a static string.

See `app/src/main/java/com/trilingua/app/ui/components/IconConventions.kt` for the authoritative reference.

## Future work

- Play Asset Delivery (split APK by language pack)
- Streaming STT (partial transcript updates while speaking)
- Language auto-detect mode
- Medium Whisper model option for better HU quality
