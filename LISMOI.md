# Outils de Bureau

Une collection d'utilitaires de bureau open-source pour l'OCR, la synthese vocale, la capture d'ecran et le traitement multimedia. La plupart des outils suivent une architecture **Rust + Python** avec activation par raccourcis clavier globaux.

## Outils

| Outil | Description | Plateformes |
|-------|-------------|-------------|
| [threshold-filter](linux/threshold-filter/) | Filtre de seuil binaire en temps reel pour le pretraitement OCR | Linux, Windows, SteamDeck |
| [voice-prompt](linux/voice-prompt/) | Dictee vocale appuyer-pour-parler (Whisper, hors ligne) | Linux, SteamDeck |
| [voice-speak](linux/voice-speak/) | Synthese vocale du texte selectionne (Piper, hors ligne) | Linux, SteamDeck |
| [screen-ocr](linux/screen-ocr/) | OCR de region d'ecran par raccourci + synthese vocale | Linux, SteamDeck |
| [deck-reader](steamdeck/deck-reader/) | Outil unifie OCR + synthese vocale multi-raccourcis | SteamDeck |
| [book-digitize](linux/book-digitize/) | OCR video-vers-markdown pour cours/livres | Linux |
| [gamebook-digitize](linux/gamebook-digitize/) | Video de livre-jeu vers HTML interactif | Linux |
| [book-reader](android/book-reader/) | Application de lecture OCR + synthese vocale | Android |
| [media-compress](linux/media-compress/) | Scripts de compression video/image/audio | Linux |
| [key-detect](linux/key-detect/) | Utilitaire de detection de codes de touches | Linux |

## Plateformes

- **[Linux](linux/)** — Outils de bureau X11 (8 outils)
- **[Windows](windows/)** — Outils compatibles Windows (1 outil)
- **[SteamDeck](steamdeck/)** — Variantes SteamOS/Wayland (5 outils)
- **[Android](android/)** — Applications Android (1 application)

## Architecture commune

Les outils bases sur des raccourcis clavier (voice-prompt, voice-speak, screen-ocr, deck-reader, threshold-filter) partagent un schema commun :

- **Ecouteur de raccourcis :** crate [rdev](https://crates.io/crates/rdev) pour les evenements clavier globaux
- **Isolation des processus :** `setsid()` + `kill(-pid, SIGKILL)` pour le nettoyage des sous-processus
- **Sortie audio :** `paplay` (PulseAudio/PipeWire)
- **Moteur TTS :** modeles ONNX [Piper](https://github.com/rhasspy/piper), entierement hors ligne
- **Moteur STT :** [faster-whisper](https://github.com/SYSTRAN/faster-whisper) (CTranslate2, CPU)
- **Moteur OCR :** [Tesseract](https://github.com/tesseract-ocr/tesseract) via pytesseract
- **Presse-papiers :** `xclip` (X11) / `wl-clipboard` (Wayland)
- **Injection de texte :** `xdotool` (X11) / `ydotool` (Wayland)
- **Configuration :** fichiers TOML dans `~/.config/<outil>/config.toml`

## Licence

[MIT](LICENSE)
