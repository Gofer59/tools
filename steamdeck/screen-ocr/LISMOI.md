# Screen OCR (SteamDeck)

OCR de region d'ecran declenchee par raccourci clavier avec synthese vocale pour SteamDeck. Concu pour les romans visuels : selectionnez la zone de dialogue une fois avec F10, puis appuyez sur F9 a chaque nouvelle ligne pour l'OCRiser, copier le texte extrait dans le presse-papier et l'entendre lu a voix haute via Piper TTS. Detecte automatiquement X11 ou Wayland et utilise les outils de capture appropries (slurp/grim sous Wayland, slop/maim sous X11). Entierement hors ligne -- Tesseract pour l'OCR, Piper pour la TTS, aucun service cloud ni cle d'API.

## Plateforme

SteamDeck (SteamOS 3.x / KDE Plasma / Wayland)

Fonctionne egalement sur les bureaux Linux standards (Debian, Ubuntu, Arch) sous X11 ou Wayland.

## Prerequis

- **Rust 1.70+** et **Cargo** (pour la compilation)
- **Python 3** (pre-installe sur SteamOS et la plupart des distributions Linux)
- Paquets systeme (installes automatiquement par `install.sh`) :
  - **Wayland (SteamDeck) :** `grim`, `slurp`, `wl-clipboard`
  - **X11 (repli) :** `maim`, `slop`, `xclip`, `xdotool`
  - `tesseract` + `tesseract-data-eng` (moteur OCR)
  - `alsa-lib` (dependance de compilation pour rdev)
- **voice-speak** (optionnel, pour la TTS) -- installez depuis le repertoire voisin `voice-speak/`

> **Systeme de fichiers SteamOS en lecture seule :** Sur SteamOS, les paquets systeme necessitent de deverrouiller temporairement le systeme de fichiers :
> ```bash
> sudo steamos-readonly disable
> sudo pacman -S --noconfirm tesseract tesseract-data-eng grim slurp wl-clipboard alsa-lib
> sudo steamos-readonly enable
> ```

## Installation

```bash
cd screen-ocr
chmod +x install.sh
./install.sh
```

L'installateur va :
1. Detecter votre serveur d'affichage (X11 ou Wayland) et gestionnaire de paquets (apt ou pacman)
2. Installer les paquets systeme pour la capture d'ecran, le presse-papier et l'OCR
3. Verifier la presence de voice-speak pour la TTS (optionnel -- l'OCR fonctionne sans)
4. Creer un venv Python a `~/.local/share/screen-ocr/venv/` avec `pytesseract` et `Pillow`
5. Compiler le binaire Rust en mode release
6. Installer le tout dans `~/.local/bin/`

Assurez-vous que `~/.local/bin` est dans votre PATH :

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### SteamOS : Installer voice-speak d'abord (recommande)

Pour le support TTS, installez l'outil voice-speak voisin avant screen-ocr :

```bash
cd ../voice-speak
chmod +x install.sh
./install.sh
```

## Utilisation

```bash
screen-ocr          # tourne au premier plan, Ctrl-C pour quitter
```

| Raccourci | Action |
|-----------|--------|
| `F10` | **Selectionner une region** -- dessinez un rectangle avec le curseur ; la geometrie est sauvegardee sur disque |
| `F9` | **Capture rapide** -- re-capture instantanee de la region sauvegardee, OCR, copie dans le presse-papier, lecture a voix haute |
| `F11` | **Arreter la TTS** -- arreter la lecture vocale |

### Flux de travail roman visuel

1. Appuyez sur **F10** pour dessiner un rectangle autour de la zone de dialogue (une seule fois)
2. Appuyez sur **F9** a chaque avancement du dialogue -- la region sauvegardee est re-capturee instantanement
3. Le texte est extrait, copie dans le presse-papier et lu a voix haute automatiquement
4. Appuyez a nouveau sur **F10** si la zone de texte se deplace ou si vous changez de jeu

### Annuler une selection

Appuyez sur **Echap** pendant que le reticule est visible (lors du F10) pour annuler.

### Demarrage automatique a la connexion

Creez `~/.config/autostart/screen-ocr.desktop` :

```ini
[Desktop Entry]
Type=Application
Name=screen-ocr
Exec=screen-ocr
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
```

## Configuration

Toute la configuration se fait en modifiant `src/main.rs` dans la structure `Config`, puis en recompilant avec `./install.sh`.

### Raccourcis clavier

```rust
quick_capture_key: Key::F9,     // re-capture la region memorisee
select_region_key: Key::F10,    // selection interactive
stop_tts_key: Key::F11,         // arreter la lecture TTS
```

### Mode de livraison

```rust
delivery_mode: DeliveryMode::Clipboard,  // "Clipboard" | "Type" | "Both"
```

| Mode | Comportement |
|------|-------------|
| `Clipboard` | Copie le texte dans le presse-papier systeme (par defaut) |
| `Type` | Tape le texte au curseur via xdotool/ydotool |
| `Both` | Copie dans le presse-papier ET tape au curseur |

### Parametres TTS

```rust
tts_voice: "en_US-lessac-medium".into(),  // modele de voix Piper
tts_speed: "1.0".into(),                   // 1.0 = normal, 1.5 = plus rapide
```

La TTS necessite que voice-speak soit installe. Sans cela, screen-ocr fonctionne normalement mais sans synthese vocale.

### Langue Tesseract

Modifiez `python/ocr_extract.py` pour changer le parametre `lang` :

```python
text = pytesseract.image_to_string(img, lang='eng').strip()       # Anglais (par defaut)
text = pytesseract.image_to_string(img, lang='jpn').strip()       # Japonais
text = pytesseract.image_to_string(img, lang='eng+fra').strip()   # Anglais + Francais
```

Installez des paquets de langue supplementaires :

```bash
sudo steamos-readonly disable
sudo pacman -S tesseract-data-jpn tesseract-data-fra
sudo steamos-readonly enable
```

Puis relancez `./install.sh` pour deployer le script mis a jour.

## Architecture

```
screen-ocr/
├── src/main.rs              Binaire Rust : ecouteur de touches, capture, dispatch OCR, presse-papier, TTS
├── python/ocr_extract.py    Script Python OCR (wrapper Tesseract : chemin image -> texte sur stdout)
├── Cargo.toml               Dependances Rust (rdev, tempfile, anyhow, ctrlc, libc, serde, serde_json)
├── requirements.txt         Dependances Python (pytesseract, Pillow)
└── install.sh               Installateur multi-distribution (apt / pacman, detection auto du serveur d'affichage)
```

**Detection auto du serveur d'affichage :**
- **X11 :** `slop` (selection) + `maim` (capture) + `xclip` (presse-papier)
- **Wayland :** `slurp` (selection) + `grim` (capture) + `wl-copy` (presse-papier)

**Modele de threading :**
- **Thread principal** : machine a etats + appels bloquants de sous-processus
- **Thread ecouteur rdev** : capture les evenements bruts du clavier, les envoie via un canal mpsc

**Fichiers installes :**

| Chemin | Role |
|--------|------|
| `~/.local/bin/screen-ocr` | Binaire compile |
| `~/.local/bin/ocr_extract.py` | Script Python OCR |
| `~/.local/bin/ocr_extract_wrapper.sh` | Wrapper venv-aware (auto-genere) |
| `~/.local/share/screen-ocr/venv/` | Environnement virtuel Python |
| `~/.local/share/screen-ocr/last_region.json` | Geometrie de region persistee |

## Notes SteamOS

- Les paquets installes via pacman (`grim`, `slurp`, `wl-clipboard`, `tesseract`) ne survivent pas aux mises a jour majeures de SteamOS -- relancez `install.sh` apres les mises a jour
- L'utilisateur doit etre dans le groupe `input` pour les raccourcis rdev : `sudo usermod -aG input $USER` (puis redemarrez)
- Le systeme de fichiers est en lecture seule par defaut ; `install.sh` gere le deverrouillage/reverrouillage via `steamos-readonly`
- Tout ce qui se trouve dans `~/.local/` (binaire, venv, fichier de region) survit aux mises a jour
- Le Mode Bureau (KDE Plasma / Wayland) est entierement supporte ; le Mode Jeu (Gamescope) peut ne pas transmettre les raccourcis a rdev

## Limitations connues

- Le Mode Jeu (compositeur Gamescope) peut ne pas transmettre les evenements clavier a rdev -- utilisez le Mode Bureau
- L'OCR Tesseract fonctionne mieux sur du texte a fort contraste avec des polices standard
- La TTS necessite que voice-speak soit installe separement
- `DeliveryMode::Type` sous Wayland necessite `ydotool` et le daemon `ydotoold`
- Les modifications de configuration necessitent de modifier `src/main.rs` et de recompiler (pas de fichier de config)
- `slurp` peut ne pas apparaitre si un jeu detient un verrou de compositeur en plein ecran exclusif

## Licence

MIT -- voir [LICENSE](../../LICENSE)
