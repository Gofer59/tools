# Outils Linux

Utilitaires de bureau pour Linux (X11). Necessite une session X11 pour les outils bases sur des raccourcis clavier.

## Outils

| Outil | Description |
|-------|-------------|
| [threshold-filter](threshold-filter/) | Filtre de seuil binaire en temps reel pour le pretraitement OCR |
| [voice-prompt](voice-prompt/) | Dictee vocale appuyer-pour-parler (Whisper, hors ligne) |
| [voice-speak](voice-speak/) | Synthese vocale du texte selectionne (Piper, hors ligne) |
| [screen-ocr](screen-ocr/) | OCR de region d'ecran par raccourci + synthese vocale |
| [book-digitize](book-digitize/) | OCR video-vers-markdown pour cours et livres |
| [gamebook-digitize](gamebook-digitize/) | Video de livre-jeu vers Markdown + HTML interactif |
| [media-compress](media-compress/) | Scripts de compression video, image et audio |
| [key-detect](key-detect/) | Utilitaire de detection de codes de touches pour configurer les raccourcis |

## Dependances communes

La plupart des outils Rust necessitent :

```bash
# Chaine d'outils Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Dependances de compilation (Ubuntu/Debian)
sudo apt-get install pkg-config libxcb-render0-dev libxcb-shape0-dev \
    libxcb-xfixes0-dev libxkbcommon-dev libgl-dev

# Dependances d'execution
sudo apt-get install xdotool xclip
```

Les outils Python utilisent des environnements virtuels crees par leurs scripts `install.sh`.
