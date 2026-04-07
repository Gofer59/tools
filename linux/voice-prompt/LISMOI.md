# voice-prompt

Dictée vocale avec touche push-to-talk qui tape vos mots **n'importe où** sous Linux —
terminaux, navigateurs, éditeurs de texte, Claude Code CLI, et tout le reste.

```
Maintenez Meta Gauche + S → parlez → relâchez S → le texte apparaît à votre curseur
```

## Architecture

```
Binaire Rust (voice-prompt)
  ├─ rdev       — écouteur global de touches (détection maintien / relâchement)
  ├─ cpal       — capture microphone → échantillons PCM f32
  ├─ hound      — écriture des échantillons dans un fichier .wav temporaire
  └─ sous-processus → python3 whisper_transcribe.py <wav> <modèle> <langue>
                         └─ faster-whisper (CTranslate2, int8, CPU)
                                └─ affiche la transcription sur stdout
                     xdotool type -- "<transcription>"
                         └─ injecte les frappes au curseur dans n'importe quelle app X11
```

## Prérequis

| Outil | Installation |
|-------|-------------|
| Rust + Cargo | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| libasound2-dev | `sudo apt install libasound2-dev` |
| xdotool | `sudo apt install xdotool` |
| python3 | pré-installé sur Linux Mint |
| faster-whisper | `pip install faster-whisper --break-system-packages` |

## Installation

```bash
chmod +x install.sh
./install.sh
```

Cette commande compile le binaire Rust en mode release et copie `voice-prompt` ainsi que `whisper_transcribe.py` dans `~/.local/bin/`.

## Utilisation

```bash
voice-prompt              # commence à écouter en anglais (par défaut) ; Ctrl-C pour quitter
voice-prompt -l fr        # reconnaissance vocale en français
voice-prompt --language fr
```

1. Focalisez n'importe quelle saisie de texte (terminal, navigateur, éditeur…)
2. Maintenez **Meta Gauche** (la touche Windows/Super gauche), appuyez sur **S**
3. Parlez naturellement
4. Relâchez **S**
5. Le texte est tapé à votre curseur en ~1–3 secondes

## Personnalisation

### Changer l'accord push-to-talk

Modifiez `src/main.rs`, `Config::modifier_key` et `Config::trigger_key` :

```rust
// Accord : Alt Gauche + S
modifier_key: Some(Key::AltLeft),  trigger_key: Key::KeyS,

// Touche unique : F9 (sans modificateur)
modifier_key: None,            trigger_key: Key::F9,

// Accord : Ctrl Droit + F
modifier_key: Some(Key::ControlRight), trigger_key: Key::KeyF,
```

Noms de touches disponibles : <https://docs.rs/rdev/latest/rdev/enum.Key.html>

Puis relancez `./install.sh`.

### Changer le modèle Whisper

Modifiez `src/main.rs`, `Config::whisper_model` :

| Modèle | Taille sur disque | Vitesse (CPU) | Précision |
|--------|-------------------|---------------|-----------|
| `"tiny"` | 75 Mo | très rapide | faible |
| `"base"` | 145 Mo | rapide | bonne |
| `"small"` | 488 Mo | moyenne | meilleure ✓ (par défaut) |
| `"medium"` | 1,5 Go | lente | excellente |
| `"large-v3"` | 3 Go | très lente | maximale |

### Sélectionner la langue de reconnaissance vocale

Utilisez l'option `-l` / `--language` :

```bash
voice-prompt -l fr        # français
voice-prompt -l en        # anglais (par défaut)
voice-prompt --language fr
```

Valeurs valides : `en` (anglais), `fr` (français).

### Démarrage automatique à la connexion

Créez `~/.config/autostart/voice-prompt.desktop` :

```ini
[Desktop Entry]
Type=Application
Name=voice-prompt
Exec=voice-prompt
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
```

## Dépannage

**`No audio input device found`**
→ Vérifiez que votre microphone est connecté et sélectionné dans le mixeur PulseAudio.
→ Essayez : `arecord -l` pour lister les périphériques.

**`xdotool: command not found`**
→ `sudo apt install xdotool`

**`faster-whisper not installed`**
→ `pip install faster-whisper --break-system-packages`

**Texte injecté dans la mauvaise fenêtre**
→ Assurez-vous que la fenêtre cible est focalisée *avant* de maintenir la touche.
→ Il y a un délai de ~1–3 s après le relâchement pendant que Python s'exécute ; ne cliquez pas ailleurs.

**`rdev` nécessite les droits root / ne fonctionne pas**
→ Sur certaines configurations, rdev a besoin d'accéder à `/dev/input`. Essayez :
→ `sudo voice-prompt`  (solution temporaire)
→ Ou ajoutez-vous au groupe `input` :
   `sudo usermod -aG input $USER` puis déconnectez-vous et reconnectez-vous.

## Note sur Wayland

`xdotool` fonctionne uniquement sous **XWayland** (la plupart des apps sur une session Wayland tournent sous XWayland par défaut sur Linux Mint 22+). Les fenêtres purement Wayland nécessitent `ydotool` à la place — remplacez `xdotool` par `ydotool` dans `inject_text()` dans `src/main.rs` et installez-le avec `sudo apt install ydotool`.
