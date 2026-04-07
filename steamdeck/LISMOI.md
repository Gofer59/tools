# Outils SteamDeck

Utilitaires concus pour SteamOS 3.x (Wayland). Ce sont des variantes compatibles Wayland des outils Linux, utilisant `slurp`/`grim` au lieu de `slop`/`maim`, et `ydotool` au lieu de `xdotool`.

## Outils

| Outil | Description |
|-------|-------------|
| [deck-reader](deck-reader/) | Outil unifie OCR + synthese vocale multi-raccourcis |
| [screen-ocr](screen-ocr/) | OCR de region d'ecran par raccourci + synthese vocale |
| [voice-prompt](voice-prompt/) | Dictee vocale appuyer-pour-parler (Whisper, hors ligne) |
| [voice-speak](voice-speak/) | Synthese vocale du texte selectionne (Piper, hors ligne) |
| [threshold-filter](threshold-filter/) | Filtre de seuil binaire en temps reel |

## Notes SteamOS

- **Systeme de fichiers en lecture seule :** les scripts d'installation gerent automatiquement `steamos-readonly disable/enable`
- **Les paquets ne survivent pas aux mises a jour :** relancez `install.sh` apres chaque mise a jour SteamOS
- **Groupe input requis :** les raccourcis globaux necessitent que l'utilisateur soit dans le groupe `input` :
  ```bash
  sudo usermod -aG input $USER
  # Puis redemarrer
  ```
- **Compiler sur une machine de developpement :** si cargo n'est pas disponible sur le Deck, compilez sur une machine Linux et copiez le binaire
