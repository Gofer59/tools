# Voice Prompt (SteamDeck)

Dictee vocale push-to-talk qui tape vos mots au curseur dans n'importe quelle application sous Linux. Maintenez une combinaison de touches, parlez naturellement, relâchez la touche, et la transcription apparait a votre curseur en quelques secondes. Utilise faster-whisper (CTranslate2) pour une reconnaissance vocale entierement hors ligne et xdotool pour l'injection de texte. Supporte l'anglais et le francais.

## Plateforme

SteamDeck (SteamOS 3.x / KDE Plasma / Wayland)

Fonctionne egalement sur les bureaux Linux standards (Debian, Ubuntu, Arch) sous X11 ou Wayland (via XWayland).

## Prerequis

- **Rust 1.70+** et **Cargo** (pour la compilation)
- **Python 3** (pre-installe sur SteamOS et la plupart des distributions Linux)
- **xdotool** (injection de texte via XWayland)
- **libasound2-dev** / **alsa-lib** (dependance de compilation pour la capture audio cpal)
- **faster-whisper** (bibliotheque Python de reconnaissance vocale)

> **Systeme de fichiers SteamOS en lecture seule :** Sur SteamOS, l'installation de paquets systeme necessite de deverrouiller temporairement le systeme de fichiers :
> ```bash
> sudo steamos-readonly disable
> sudo pacman -S --noconfirm xdotool alsa-lib python
> sudo steamos-readonly enable
> ```

> **Note Wayland :** `xdotool` fonctionne sous XWayland (la plupart des applications sur KDE Plasma tournent sous XWayland par defaut). Pour les fenetres purement Wayland, remplacez `xdotool` par `ydotool` dans `inject_text()` dans `src/main.rs` et installez `ydotool`.

## Installation

```bash
cd voice-prompt
chmod +x install.sh
./install.sh
```

L'installateur va :
1. Verifier les dependances systeme (cargo, python3, xdotool)
2. S'assurer que les en-tetes de developpement ALSA sont installes
3. Installer le paquet Python faster-whisper
4. Compiler le binaire Rust en mode release
5. Installer `voice-prompt` et `whisper_transcribe.py` dans `~/.local/bin/`

Assurez-vous que `~/.local/bin` est dans votre PATH :

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## Utilisation

```bash
voice-prompt              # Reconnaissance vocale en anglais (par defaut)
voice-prompt -l fr        # Reconnaissance vocale en francais
voice-prompt --language fr
```

| Action | Combinaison de touches |
|--------|----------------------|
| **Demarrer l'enregistrement** | Maintenez **Meta Gauche** (Super), appuyez sur **S** |
| **Arreter + transcrire** | Relâchez **S** (Meta peut rester maintenu) |

1. Focalisez n'importe quelle saisie de texte (terminal, navigateur, editeur, etc.)
2. Maintenez **Meta Gauche**, appuyez sur **S** -- l'enregistrement demarre
3. Parlez naturellement
4. Relâchez **S** -- l'enregistrement s'arrete, la transcription s'execute
5. Le texte apparait a votre curseur en ~1--3 secondes

## Configuration

Toute la configuration se fait en modifiant `src/main.rs` dans la structure `Config`, puis en recompilant avec `./install.sh`.

### Accord push-to-talk

```rust
// Par defaut : Meta Gauche + S
modifier_key: Some(Key::MetaLeft),  trigger_key: Key::KeyS,

// Alternative : touche unique F9 (sans modificateur)
modifier_key: None,                 trigger_key: Key::F9,

// Alternative : Alt Gauche + S
modifier_key: Some(Key::Alt),       trigger_key: Key::KeyS,
```

### Modele Whisper

Modifiez `Config::whisper_model` :

| Modele | Taille | Vitesse (CPU) | Precision |
|--------|--------|---------------|-----------|
| `"tiny"` | 75 Mo | tres rapide | faible |
| `"base"` | 145 Mo | rapide | bonne |
| `"small"` | 488 Mo | moyenne | meilleure (par defaut) |
| `"medium"` | 1,5 Go | lente | excellente |
| `"large-v3"` | 3 Go | tres lente | maximale |

### Langue

Utilisez l'option CLI `-l` / `--language` :

```bash
voice-prompt -l en        # Anglais (par defaut)
voice-prompt -l fr        # Francais
```

### Demarrage automatique a la connexion

Creez `~/.config/autostart/voice-prompt.desktop` :

```ini
[Desktop Entry]
Type=Application
Name=voice-prompt
Exec=voice-prompt
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
```

## Architecture

```
voice-prompt/
├── src/main.rs                  Binaire Rust : machine a etats push-to-talk, capture audio, dispatch sous-processus
├── python/whisper_transcribe.py Script Python de transcription (faster-whisper, CTranslate2, int8, CPU)
├── Cargo.toml                   Dependances Rust (cpal, hound, rdev, tempfile, anyhow, ctrlc)
└── install.sh                   Script de compilation + installation
```

**Modele de threading :**
- **Thread principal** : machine a etats (Idle / Recording) + appel du sous-processus Python
- **Thread ecouteur rdev** : capture les evenements bruts du clavier, les envoie via un canal mpsc
- **Thread stream cpal** : pousse les echantillons audio (PCM f32) via un autre canal

**Flux de donnees :**
1. cpal capture l'entree microphone en echantillons PCM f32
2. Les echantillons sont convertis en i16 et ecrits dans un fichier WAV temporaire (hound)
3. Le script Python execute faster-whisper sur le fichier WAV, affiche la transcription sur stdout
4. xdotool tape la transcription a la position courante du curseur

**Fichiers installes :**

| Chemin | Role |
|--------|------|
| `~/.local/bin/voice-prompt` | Binaire compile |
| `~/.local/bin/whisper_transcribe.py` | Script Python de transcription |

## Notes SteamOS

- Les paquets installes via pacman (`xdotool`, `alsa-lib`) ne survivent pas aux mises a jour majeures de SteamOS -- relancez `install.sh` apres les mises a jour
- L'utilisateur doit etre dans le groupe `input` pour les raccourcis rdev : `sudo usermod -aG input $USER` (puis redemarrez)
- Le systeme de fichiers est en lecture seule par defaut ; deverrouillez avec `sudo steamos-readonly disable` avant d'installer les paquets systeme
- La chaine d'outils Rust (`~/.rustup/`, `~/.cargo/`) et les fichiers installes (`~/.local/bin/`) survivent aux mises a jour
- `xdotool` fonctionne sous XWayland sur KDE Plasma ; pour les fenetres purement Wayland, utilisez `ydotool` a la place

## Limitations connues

- `xdotool` ne fonctionne que sous XWayland -- les fenetres purement Wayland necessitent `ydotool` (modification de code necessaire)
- Le delai de transcription de ~1--3 secondes signifie que vous ne devez pas cliquer en dehors de la fenetre cible avant que le texte n'apparaisse
- Le telechargement du modele Whisper s'effectue au premier lancement du script Python et peut etre lent (~500 Mo pour "small")
- Pas de fichier de config -- tous les reglages necessitent de modifier `src/main.rs` et de recompiler
- La capture audio utilise le peripherique d'entree par defaut du systeme (configure dans le mixeur PulseAudio/PipeWire)
- Le Mode Jeu (Gamescope) peut ne pas transmettre les evenements clavier a rdev -- utilisez le Mode Bureau

## Licence

MIT -- voir [LICENSE](../../LICENSE)
