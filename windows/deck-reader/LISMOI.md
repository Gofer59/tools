# Deck Reader (Windows)

Outil multi-raccourcis d'OCR d'ecran + synthese vocale pour Windows 10/11. Ecoute globalement trois combinaisons de touches configurables : une pour selectionner interactivement une region d'ecran et l'OCRiser, une pour re-capturer instantanement la derniere region sauvegardee, et une pour basculer la TTS sur le texte du presse-papier. Concu pour lire a voix haute les dialogues de romans visuels -- selectionnez la zone de texte une fois, puis appuyez sur une seule touche a chaque nouvelle ligne. Entierement hors ligne : Tesseract pour l'OCR, Piper pour la TTS, aucun service cloud ni cle d'API requis.

## Plateforme

- **Windows 10 / Windows 11** -- support MVP (presse-papiers uniquement, pas de daemon TTS persistant)

## Prerequis

- **Python 3.10+** -- [python.org](https://www.python.org/downloads/) (ajouter au PATH lors de l'installation)
- **Rust / cargo** -- [rustup.rs](https://rustup.rs/)
- **winget** -- inclus dans Windows 10 1809+ / Windows 11 (mettre a jour via Microsoft Store -> App Installer)

## Installation

Depuis une session PowerShell elevee (clic droit -> "Executer en tant qu'administrateur") :

```powershell
cd deck-reader
.\install.bat
```

Ou sans le wrapper :

```powershell
Set-ExecutionPolicy Bypass -Scope Process
.\install.ps1
```

L'installateur va :
1. Verifier les prerequis (Python, Rust, winget)
2. Installer Tesseract OCR via `winget install UB-Mannheim.TesseractOCR`
3. Compiler le binaire Rust (`cargo build --release`)
4. Copier le binaire + scripts wrapper dans `%LOCALAPPDATA%\deck-reader\bin\`
5. Creer un venv Python a `%LOCALAPPDATA%\deck-reader\venv\` et installer les dependances
6. Copier les scripts Python dans `%LOCALAPPDATA%\deck-reader\python\`
7. Telecharger le modele vocal Piper `en_US-lessac-medium` (~65 Mo depuis HuggingFace)
8. Ecrire une config par defaut dans `%APPDATA%\deck-reader\config.toml`
9. Creer un raccourci dans le menu Demarrer

### Ignorer le telechargement du modele

Si vous avez deja le modele ou souhaitez le telecharger manuellement :

```powershell
.\install.ps1 -SkipModel
```

## Utilisation

| Raccourci | Action |
|-----------|--------|
| `Alt + U` | Selectionner une region -- overlay plein ecran, dessiner un rectangle, OCR + presse-papiers |
| `Alt + I` | Re-OCR de la derniere region sauvegardee -> presse-papiers |
| `Alt + Y` | Bascule TTS -- lit le texte du presse-papiers, ou arrete si deja en lecture |

### Flux de travail roman visuel

1. Lancez le jeu (fenetre ou plein ecran)
2. Demarrez `deck-reader` depuis le menu Demarrer ou un terminal
3. Appuyez sur `Alt+U` pour dessiner un rectangle autour de la zone de dialogue
4. Appuyez sur `Alt+I` a chaque avancement du dialogue (pas besoin de re-dessiner)
5. Appuyez sur `Alt+Y` pour arreter la lecture en cours ou pour lire du texte du presse-papiers

### Mode detection de touches

Decouvrez les codes de touches bruts pour le mappage des boutons :

```powershell
deck-reader --detect-keys
```

## Configuration

Fichier de config : `%APPDATA%\deck-reader\config.toml`

Cree automatiquement avec les valeurs par defaut au premier lancement. Modifiez avec n'importe quel editeur de texte ; redemarrez `deck-reader` pour que les changements prennent effet.

```toml
[hotkeys]
tts_toggle  = "Alt+KeyY"
ocr_select  = "Alt+KeyU"
ocr_capture = "Alt+KeyI"

[tts]
voice = "en_US-lessac-medium"   # Nom du modele Piper (doit exister dans le dossier models)
speed = 1.0                     # 1.0=normal, 1.5=plus rapide, 0.8=plus lent

[ocr]
language      = "eng"           # Codes de langue Tesseract : "eng", "eng+jpn", etc.
delivery_mode = "clipboard"     # "clipboard" uniquement sur Windows (MVP)
cleanup       = true            # Nettoyer les artefacts OCR (symboles parasites, ponctuation repetee)
```

### Format des raccourcis

- Touches nommees : `MetaLeft`, `AltLeft`, `KeyA`--`KeyZ`, `F1`--`F12`, `Space`, `Return`, etc.
- Combinaisons : `"Alt+KeyU"`, `"MetaLeft+F9"`

### Changer la voix Piper

Telechargez les fichiers `.onnx` et `.onnx.json` depuis le [depot HuggingFace de Piper](https://huggingface.co/rhasspy/piper-voices) vers `%LOCALAPPDATA%\deck-reader\models\`, puis definissez `voice` dans `config.toml` sur le nom du modele (nom de fichier sans extension).

## Fichiers installes

| Chemin | Role |
|--------|------|
| `%LOCALAPPDATA%\deck-reader\bin\deck-reader.exe` | Binaire compile |
| `%LOCALAPPDATA%\deck-reader\bin\ocr_extract_wrapper.bat` | Wrapper OCR |
| `%LOCALAPPDATA%\deck-reader\venv\` | Environnement virtuel Python |
| `%LOCALAPPDATA%\deck-reader\models\` | Modeles vocaux ONNX Piper |
| `%LOCALAPPDATA%\deck-reader\python\` | Scripts Python |
| `%LOCALAPPDATA%\deck-reader\last_region.json` | Region OCR persistee |
| `%APPDATA%\deck-reader\config.toml` | Configuration |

## Limitations connues (MVP)

- `delivery_mode = "clipboard"` uniquement -- l'injection de texte (`"type"` / `"both"`) n'est pas encore implementee
- Pas de daemon TTS persistant -- chaque requete TTS subit un delai de demarrage Piper de ~3-5 s
- Pas de fenetre de statut GUI -- fonctionnement en mode console uniquement
- La selection de region multi-ecran est limitee a l'ecran contenant le point de depart du glisser

## Avertissement SmartScreen

Le binaire non signe declenchera Windows Defender SmartScreen au premier lancement.
Pour contourner : clic droit sur `deck-reader.exe` -> Proprietes -> cocher **Debloquer** -> OK.
Ou lancez depuis un terminal -- SmartScreen ne s'active que pour les lancements GUI.

## Licence

MIT -- voir [LICENSE](../../LICENSE)
