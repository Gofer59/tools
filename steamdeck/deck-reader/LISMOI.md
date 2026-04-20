# Deck Reader (SteamDeck)

Outil multi-raccourcis d'OCR d'ecran + synthese vocale pour le Mode Bureau du SteamDeck. Ecoute globalement trois combinaisons de touches configurables : une pour selectionner interactivement une region d'ecran et l'OCRiser, une pour re-capturer instantanement la derniere region sauvegardee, et une pour basculer la TTS sur le texte surligne ou le presse-papier. Concu pour lire a voix haute les dialogues de romans visuels -- selectionnez la zone de texte une fois, puis appuyez sur une seule touche a chaque nouvelle ligne. Entierement hors ligne : Tesseract pour l'OCR, Piper pour la TTS, aucun service cloud ni cle d'API requis.

## Plateforme

- **SteamDeck / SteamOS 3.x** — cible principale (KDE Plasma / Wayland)
- **Windows 10 / Windows 11** — support MVP (presse-papiers uniquement, pas de daemon TTS persistant)

## Prerequis

- **Rust 1.70+** sur votre machine de developpement (non requis sur le SteamDeck)
- **Python 3** (pre-installe sur SteamOS)
- Paquets systeme installes via pacman (geres par `install.sh`) :
  - `wl-clipboard` (fournit `wl-paste`, `wl-copy`)
  - `grim` (capture d'ecran Wayland)
  - `slurp` (selection interactive de region Wayland)
  - `tesseract` + `tesseract-data-eng` (moteur OCR)
  - `tk` (GUI Python pour la fenetre de statut)

> **Systeme de fichiers SteamOS en lecture seule :** L'installateur deverrouille temporairement le systeme de fichiers avec `steamos-readonly disable`, installe les paquets, puis le reverrouille. Vous n'avez pas besoin de le faire manuellement.

## Installation

### Etape 1 : Compiler sur votre machine de developpement

Sur n'importe quel PC Linux x86_64 avec Rust installe :

```bash
cd deck-reader
cargo build --release
```

### Etape 2 : Copier sur le SteamDeck

Copiez le dossier entier `deck-reader/` (incluant `target/release/deck-reader`) sur le SteamDeck :

```bash
scp -r deck-reader/ deck@steamdeck:~/deck-reader/
```

### Etape 3 : Lancer l'installateur

Sur le SteamDeck, en Mode Bureau :

```bash
cd ~/deck-reader
chmod +x install.sh
./install.sh
```

L'installateur va :
1. Deverrouiller le systeme de fichiers SteamOS et installer les paquets systeme via pacman
2. Reverrouiller le systeme de fichiers
3. Verifier/proposer d'ajouter votre utilisateur au groupe `input`
4. Verifier que le binaire pre-compile existe
5. Creer un venv Python a `~/.local/share/deck-reader/venv/` et installer les dependances
6. Telecharger le modele vocal Piper `en_US-lessac-medium` (~65 Mo)
7. Installer les fichiers dans `~/.local/bin/` et generer les scripts wrapper
8. Creer une entree dans le menu d'applications KDE

### Etape 4 : Configurer le PATH

Ajoutez a `~/.bashrc` si ce n'est pas deja fait :

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Etape 5 : Lancer

```bash
deck-reader
```

Ou cherchez "Deck Reader" dans le menu d'applications KDE.

## Utilisation

| Raccourci | Action |
|-----------|--------|
| `Alt + U` | **Selectionner une region** -- dessinez un rectangle, OCR, copie dans le presse-papier, lecture a voix haute |
| `Alt + I` | **Re-capturer** -- re-OCR instantane de la derniere region sauvegardee, lit le nouveau texte |
| `Alt + Y` | **Bascule TTS** -- lit le texte surligne/presse-papier, ou arrete si deja en lecture |

### Flux de travail roman visuel

1. Lancez le jeu en Mode Bureau (fenetre ou plein ecran)
2. Demarrez `deck-reader` depuis le menu d'applications KDE ou un terminal
3. Appuyez sur `Alt+U` pour dessiner un rectangle autour de la zone de dialogue
4. Appuyez sur `Alt+I` a chaque avancement du dialogue (pas besoin de re-dessiner)
5. Appuyez sur `Alt+Y` pour arreter la lecture en cours ou pour lire du texte selectionne

### Mode detection de touches

Decouvrez les codes de touches bruts pour le mappage des boutons Steam Input :

```bash
deck-reader --detect-keys
```

Appuyez sur n'importe quel bouton pour voir son code, puis utilisez ce code dans `config.toml`.

## Configuration

Fichier de config : `~/.config/deck-reader/config.toml`

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
delivery_mode = "clipboard"     # "clipboard" | "type" | "both"
cleanup       = true            # Nettoyer les artefacts OCR (symboles parasites, ponctuation repetee)

[paths]
# Surcharges optionnelles (valeurs par defaut affichees) :
# models_dir  = "~/.local/share/deck-reader/models"
# venv_dir    = "~/.local/share/deck-reader/venv"
# region_file = "~/.local/share/deck-reader/last_region.json"
```

### Format des raccourcis

- Touches nommees : `MetaLeft`, `AltLeft`, `KeyA`--`KeyZ`, `F1`--`F12`, `Space`, `Return`, etc.
- Combinaisons : `"Alt+KeyU"`, `"MetaLeft+F9"`
- Codes de touches bruts depuis Steam Input : `"191"` ou `"Unknown(191)"`

### Ajouter des paquets de langue Tesseract

```bash
sudo steamos-readonly disable
sudo pacman -S tesseract-data-jpn tesseract-data-fra
sudo steamos-readonly enable
```

Puis definissez `language = "jpn"` ou `language = "eng+jpn"` dans `config.toml`.

### Changer la voix Piper

Telechargez les fichiers `.onnx` et `.onnx.json` depuis le [depot HuggingFace de Piper](https://huggingface.co/rhasspy/piper-voices) vers `~/.local/share/deck-reader/models/`, puis definissez `voice` dans `config.toml` sur le nom du modele (nom de fichier sans extension).

## Architecture

```
deck-reader/
├── src/main.rs           Binaire Rust : ecouteur de touches, machine a etats, dispatch de sous-processus
├── python/
│   ├── ocr_extract.py    Wrapper OCR Tesseract (chemin image -> texte sur stdout)
│   ├── tts_speak.py      Synthese TTS Piper + lecture paplay
│   ├── tts_daemon.py     Daemon TTS persistant (socket Unix, demarrage plus rapide)
│   └── gui_window.py     Fenetre GUI de statut
├── Cargo.toml            Dependances Rust (rdev, anyhow, toml, serde, serde_json, ...)
├── requirements.txt      Dependances Python (piper-tts, pytesseract, Pillow, ...)
└── install.sh            Installateur SteamDeck (pacman, venv, telechargement modele, entree menu)
```

**Modele de threading :**
- **Thread principal** : machine a etats + appels bloquants de sous-processus
- **Thread ecouteur rdev** : capture les evenements bruts du clavier depuis `/dev/input`, les envoie via un canal mpsc

**Isolation des sous-processus :** Les processus TTS sont lances avec `setsid()` dans leur propre groupe de processus. L'arret de la lecture envoie `SIGKILL` au PID negatif, tuant instantanement le groupe entier (shell, Python, paplay).

**Fichiers installes :**

| Chemin | Role |
|--------|------|
| `~/.local/bin/deck-reader` | Binaire compile |
| `~/.local/bin/tts_speak_wrapper.sh` | Wrapper TTS venv-aware (auto-genere) |
| `~/.local/bin/ocr_extract_wrapper.sh` | Wrapper OCR venv-aware (auto-genere) |
| `~/.config/deck-reader/config.toml` | Configuration (cree au premier lancement) |
| `~/.local/share/deck-reader/venv/` | Environnement virtuel Python |
| `~/.local/share/deck-reader/models/` | Modeles vocaux ONNX Piper |
| `~/.local/share/deck-reader/last_region.json` | Geometrie de region OCR persistee |
| `~/.local/share/applications/deck-reader.desktop` | Entree menu d'applications KDE |

## Notes SteamOS

- Les paquets installes via pacman (`wl-clipboard`, `grim`, `slurp`, `tesseract`) ne survivent pas aux mises a jour majeures de SteamOS -- relancez `install.sh` apres les mises a jour
- L'utilisateur doit etre dans le groupe `input` pour les raccourcis rdev : `sudo usermod -aG input $USER` (puis redemarrez)
- Le systeme de fichiers est en lecture seule par defaut ; `install.sh` gere le deverrouillage/reverrouillage via `steamos-readonly`
- Tout ce qui se trouve dans `~/.local/` et `~/.config/` (binaire, venv, modeles, config) survit aux mises a jour
- L'appartenance au groupe `input` survit aux mises a jour
- Le Mode Jeu (Gamescope) peut ne pas transmettre les raccourcis a rdev -- utilisez le Mode Bureau pour de meilleurs resultats

### Survivre a une mise a jour SteamOS

Lorsque SteamOS applique une mise a jour majeure, deux choses cassent l'installateur :

1. Les paquets systeme installes via pacman sont effaces.
2. Le trousseau de cles pacman dans `/etc/pacman.d/gnupg` est reinitialise / non
   inscriptible, donc tout appel `pacman -S` echoue avec des erreurs du type :

   ```
   warning: Public keyring not found; have you run 'pacman-key --init'?
   error: keyring is not writable
   error: required key missing from keyring
   error: failed to commit transaction (unexpected error)
   ```

L'actuel `install.sh` gere les deux automatiquement dans les etapes 1 a 4.

#### Recommande : cliquer sur l'entree de reparation

`install.sh` installe un lanceur dedie « **Deck Reader — Post-update fix** »
dans votre menu d'applications KDE. Apres une mise a jour SteamOS, cherchez-le
dans le menu d'applications (ou ouvrez Dolphin dans `~/deck-reader/` et
double-cliquez `post-update-fix.desktop`) — il ouvre une Konsole, demande votre
mot de passe sudo une fois, et execute toute la recuperation (deverrouillage →
init du trousseau → installation pacman → reverrouillage). Pas besoin de
relancer `install.sh` ni de retelecharger quoi que ce soit.

#### Recuperation manuelle (equivalente)

```bash
sudo steamos-readonly disable
sudo pacman-key --init
sudo pacman-key --populate archlinux
sudo pacman-key --populate holo           # peut ne pas exister sur toutes les images SteamOS
sudo pacman -S --needed \
    xclip xdotool maim slop wl-clipboard grim slurp \
    tesseract tesseract-data-eng tk
sudo steamos-readonly enable
```

Votre venv, vos modeles, votre config et votre binaire dans `~/.local/` et
`~/.config/` sont intacts apres les mises a jour, donc vous n'avez jamais
besoin de refaire les etapes 6 a 10.

## Limitations connues

- Necessite le Mode Bureau (KDE Plasma) -- Gamescope en Mode Jeu peut ne pas transmettre les evenements clavier a rdev
- L'OCR Tesseract fonctionne mieux sur du texte a fort contraste avec des polices standard ; les polices a faible contraste ou stylisees peuvent donner de mauvais resultats
- `delivery_mode = "type"` necessite que le daemon `ydotoold` soit en cours d'execution (`systemctl --user start ydotoold`)
- Les applications Electron (Discord, VS Code) ne renseignent pas la selection PRIMARY Wayland -- utilisez Ctrl+C d'abord, puis Alt+Y
- Le binaire doit etre compile en croix sur une machine de developpement (SteamOS ne possede pas les en-tetes de developpement par defaut)
- `slurp` peut ne pas apparaitre si un jeu detient un verrou de compositeur en plein ecran exclusif

---

## Installation Windows

### Prerequis

- **Python 3.10+** — [python.org](https://www.python.org/downloads/) (ajouter au PATH lors de l'installation)
- **Rust / cargo** — [rustup.rs](https://rustup.rs/)
- **winget** — inclus dans Windows 10 1809+ / Windows 11 (mettre a jour via Microsoft Store → App Installer)

### Installer

Depuis une session PowerShell elevee (clic droit → « Executer en tant qu'administrateur ») :

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

### Utilisation (Windows)

| Raccourci | Action |
|-----------|--------|
| `Alt + U` | Selectionner une region — overlay plein ecran, dessiner un rectangle, OCR + presse-papiers |
| `Alt + I` | Re-OCR de la derniere region sauvegardee → presse-papiers |
| `Alt + Y` | Bascule TTS — lit le texte du presse-papiers, ou arrete si deja en lecture |

### Limitations Windows (MVP)

- `delivery_mode = "clipboard"` uniquement — l'injection de texte (`"type"` / `"both"`) n'est pas encore implementee
- Pas de daemon TTS persistant — chaque requete TTS subit un delai de demarrage Piper de ~3–5 s
- Pas de fenetre de statut GUI — fonctionnement en mode console uniquement
- La selection de region multi-ecran est limitee a l'ecran contenant le point de depart du glisser

### Fichiers installes (Windows)

| Chemin | Role |
|--------|------|
| `%LOCALAPPDATA%\deck-reader\bin\deck-reader.exe` | Binaire compile |
| `%LOCALAPPDATA%\deck-reader\bin\ocr_extract_wrapper.bat` | Wrapper OCR |
| `%LOCALAPPDATA%\deck-reader\venv\` | Environnement virtuel Python |
| `%LOCALAPPDATA%\deck-reader\models\` | Modeles vocaux ONNX Piper |
| `%LOCALAPPDATA%\deck-reader\python\` | Scripts Python |
| `%LOCALAPPDATA%\deck-reader\last_region.json` | Region OCR persistee |
| `%APPDATA%\deck-reader\config.toml` | Configuration |

### Avertissement SmartScreen

Le binaire non signe declenchera Windows Defender SmartScreen au premier lancement.
Pour contourner : clic droit sur `deck-reader.exe` → Proprietes → cocher **Debloquer** → OK.
Ou lancez depuis un terminal — SmartScreen ne s'active que pour les lancements GUI.

---

## Licence

MIT -- voir [LICENSE](../../LICENSE)
