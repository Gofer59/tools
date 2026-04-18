# Trilingua

Traducteur vocal hors-ligne pour Android : Anglais, Français et Hongrois — les 6 paires directes.

## Fonctionnalités

- Bouton micro à maintenir enfoncé
- 3 langues + 6 paires de traduction sans pivot
- Zéro connexion réseau (aucune permission internet)
- ~881 Mo (tous les modèles intégrés)

## Paires supportées

| # | Direction | STT | Modèle MT | Voix TTS |
|---|-----------|-----|-----------|----------|
| 1 | EN → HU | whisper small q5_1 | opus-mt-en-hu int8 | hu_HU-anna-medium |
| 2 | HU → EN | whisper small q5_1 | opus-mt-hu-en int8 | en_US-lessac-medium |
| 3 | FR → HU | whisper small q5_1 | opus-mt-fr-hu int8 | hu_HU-anna-medium |
| 4 | HU → FR | whisper small q5_1 | opus-mt-hu-fr int8 | fr_FR-siwis-medium |
| 5 | EN → FR | whisper small q5_1 | opus-mt-en-fr int8 | fr_FR-siwis-medium |
| 6 | FR → EN | whisper small q5_1 | opus-mt-fr-en int8 | en_US-lessac-medium |

## Installation

1. Activer « Sources inconnues » sur l'appareil Android (Paramètres → Sécurité)
2. `adb install -r app/build/outputs/apk/debug/app-debug.apk`
3. Accorder la permission microphone au premier lancement
4. Attendre la progression d'extraction (~30 s au premier lancement)

## Permissions

- `RECORD_AUDIO` — microphone push-to-talk
- Pas de permission internet — inférence 100% hors ligne

## Taille de l'APK

| Catégorie | Taille |
|-----------|--------|
| STT (whisper ggml-small-q5_1) | ~190 Mo |
| MT (6 × opus-mt int8) | ~360 Mo |
| TTS (3 × voix Piper) | ~180 Mo |
| Code + bibliothèques natives | ~20 Mo |
| **Total** | **~881 Mo** |

## Compilation

### Prérequis

- JDK 21 (avec jlink, ex. `~/jdk-21.0.2`)
- Android SDK Platform 34 + Build-Tools 34.0.0
- Android NDK r27.1.12297006
- Android CMake 3.22.1
- Python 3.11+ (pour la conversion des modèles)
- `curl`, `git`

### Étapes

```bash
cd tools/trilingua

# 1. Télécharger et convertir les modèles (~881 Mo, ~15 min)
bash scripts/download-models.sh

# 2. Compiler les bibliothèques JNI (nécessite NDK)
export ANDROID_NDK_HOME=/chemin/vers/Android/Sdk/ndk/27.1.12297006
bash scripts/build-native.sh

# 3. Assembler l'APK
./gradlew assembleDebug
# APK : app/build/outputs/apk/debug/app-debug.apk

# 4. Installer sur l'appareil
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

## Architecture

```
Application Android (Kotlin + Jetpack Compose)
├── STT : whisper.cpp (JNI) — modèle multilingue small
├── MT :  CTranslate2 (JNI) + SentencePiece — 6 modèles OPUS-MT int8
└── TTS : sherpa-onnx OfflineTts — voix Piper VITS pour EN/FR/HU
```

## Limitations connues

- **Qualité STT Hongrois** : la précision de whisper small q5_1 sur le hongrois n'est pas mesurée. Remplacer par `ggml-medium-q5_0.bin` si nécessaire.
- **Artefacts vocaux hu_HU-anna** : remplacer par `hu_HU-berta-medium` dans `scripts/download-models.sh`.
- **Taille ~881 Mo** : sideload uniquement pour v1. Migration Play Asset Delivery prévue.
- **arm64-v8a uniquement** : pas de support x86_64 ni armeabi-v7a.
