# voice-speak

Synthèse vocale pour le texte sélectionné — appuyez sur un raccourci clavier et entendez-le lu à voix haute.

```
Sélectionnez du texte → appuyez sur Alt Droite → le son sort par vos haut-parleurs
Appuyez à nouveau sur Alt Droite pendant la lecture → arrêt immédiat
```

Fonctionne dans n'importe quelle application sous Linux (terminaux, navigateurs, PDF, éditeurs de code…).

## Architecture

```
Binaire Rust (voice-speak)
  ├─ rdev          — écouteur global de touches (bascule : parler / arrêter)
  ├─ xclip         — lit la sélection PRIMARY (texte surligné), repli sur CLIPBOARD
  └─ sous-processus → tts_speak_wrapper.sh  (active l'environnement virtuel Python)
                       └─ python3 tts_speak.py <texte> <voix> <vitesse>
                            ├─ piper-tts   (TTS neural ONNX, CPU, entièrement hors ligne)
                            └─ paplay      (achemine via PulseAudio/PipeWire)
```

Le sous-processus est lancé dans son propre groupe de processus (`setsid`), de sorte qu'appuyer sur le raccourci pour arrêter envoie `SIGKILL` à tout le groupe — Python et `paplay` s'arrêtent instantanément.

## Prérequis

| Outil | Installation |
|-------|-------------|
| Rust + Cargo | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| xclip | `sudo apt install xclip` (X11) |
| wl-clipboard | `sudo apt install wl-clipboard` (Wayland) |
| python3 | pré-installé sur Linux Mint |
| paplay | pré-installé (inclus dans `pulseaudio-utils`) |

Les dépendances Python (`piper-tts`, `sounddevice`, `numpy`) sont installées automatiquement dans un environnement virtuel isolé par `install.sh`.

## Installation

```bash
chmod +x install.sh
./install.sh
```

Cette commande :
1. Compile le binaire Rust en mode release
2. Crée un environnement virtuel Python à `~/.local/share/voice-speak/venv/`
3. Télécharge le modèle de voix Piper par défaut (`en_US-lessac-medium`) vers `~/.local/share/voice-speak/models/`
4. Copie tout dans `~/.local/bin/`

## Utilisation

```bash
voice-speak          # commence à écouter ; Ctrl-C pour quitter
```

1. Sélectionnez / surlignez du texte dans n'importe quelle application
2. Appuyez sur **Alt Droite**
3. Le texte est lu à voix haute (~0,5–1 s de latence pour les courtes phrases)
4. Pour arrêter en cours de lecture : appuyez à nouveau sur **Alt Droite**

## Personnalisation

Tous les réglages sont dans `src/main.rs` à l'intérieur de `Config::default()`. Après toute modification, relancez `./install.sh` pour recompiler et réinstaller.

### Changer le raccourci clavier

```rust
hotkey: Key::AltGr,        // Alt Droite (par défaut)
hotkey: Key::F10,          // F10
hotkey: Key::ControlRight, // Ctrl Droit
```

Noms de touches disponibles : https://docs.rs/rdev/latest/rdev/enum.Key.html

### Changer la voix

```rust
voice: "en_US-lessac-medium".into(),   // par défaut
voice: "en_US-ryan-medium".into(),     // voix masculine plus expressive
voice: "en_GB-alan-medium".into(),     // voix masculine britannique
```

Le modèle doit être téléchargé au préalable (voir **Voix** ci-dessous). Aucune modification de code n'est nécessaire si vous souhaitez simplement tester une voix — vous pouvez appeler le script Python directement :

```bash
~/.local/share/voice-speak/venv/bin/python3 \
  ~/.local/bin/tts_speak.py "Bonjour monde" en_US-ryan-medium 1.0
```

### Changer la vitesse de la parole

```rust
speed: 1.0,    // normale (par défaut)
speed: 1.5,    // 50 % plus rapide
speed: 0.8,    // 20 % plus lent
```

### Paramètres de synthèse avancés (`tts_speak.py`)

Modifiez `SynthesisConfig` dans `tts_speak.py` (pas besoin de recompiler, copiez simplement le fichier vers `~/.local/bin/`) :

```python
syn_config = SynthesisConfig(
    length_scale=1.0 / speed,   # durée des phonèmes ; <1 = plus rapide, >1 = plus lent
    noise_scale=0.667,           # expressivité / naturel (0 = robotique, 1 = varié)
    noise_w_scale=0.8,           # variation du rythme (0 = plat/uniforme, 1 = naturel)
)
```

**À propos des pauses** (virgules, points, etc.) : Piper génère les pauses en interne en fonction de la ponctuation — il n'existe pas de paramètre de pause dédié. Pour raccourcir les pauses, augmentez `speed` ou réduisez `noise_w_scale` vers 0 pour un rythme plus uniforme.

## Voix

Piper propose des dizaines de voix neurales gratuites et hors ligne. Parcourez la liste complète sur :
https://huggingface.co/rhasspy/piper-voices/tree/v1.0.0

### Voix anglaises recommandées

| Nom du modèle | Style | Taille |
|---|---|---|
| `en_US-lessac-medium` | masculin neutre (par défaut) | ~60 Mo |
| `en_US-ryan-medium` | masculin expressif | ~60 Mo |
| `en_US-ljspeech-medium` | féminin clair | ~60 Mo |
| `en_GB-alan-medium` | masculin britannique | ~60 Mo |
| `en_GB-jenny_dioco-medium` | féminin britannique | ~60 Mo |
| `en_US-lessac-high` | masculin neutre, meilleure qualité | ~130 Mo |

### Télécharger une nouvelle voix

Chaque voix nécessite deux fichiers : `.onnx` (modèle) et `.onnx.json` (config).

```bash
# Exemple : en_US-ryan-medium
BASE="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/ryan/medium"
MODEL_DIR="$HOME/.local/share/voice-speak/models"

curl -L -o "$MODEL_DIR/en_US-ryan-medium.onnx"      "$BASE/en_US-ryan-medium.onnx"
curl -L -o "$MODEL_DIR/en_US-ryan-medium.onnx.json" "$BASE/en_US-ryan-medium.onnx.json"
```

Modèle d'URL : `.../piper-voices/resolve/v1.0.0/<langue>/<langue_région>/<nom>/<qualité>/<fichier>`

### Voix françaises

```bash
BASE="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/fr/fr_FR/siwis/medium"
curl -L -o "$MODEL_DIR/fr_FR-siwis-medium.onnx"      "$BASE/fr_FR-siwis-medium.onnx"
curl -L -o "$MODEL_DIR/fr_FR-siwis-medium.onnx.json" "$BASE/fr_FR-siwis-medium.onnx.json"
```

Puis définissez `voice: "fr_FR-siwis-medium".into()` dans `src/main.rs`.

## Structure des fichiers

```
voice-speak/
├── Cargo.toml            # rdev, anyhow, ctrlc, libc
├── src/main.rs           # Binaire Rust : boucle de touches, lecture du presse-papier, gestion des sous-processus
├── tts_speak.py          # Python : synthèse Piper + lecture avec paplay
├── requirements.txt      # piper-tts, sounddevice, numpy
└── install.sh            # script d'installation en une étape

Installé dans :
~/.local/bin/
  voice-speak             # Binaire Rust
  tts_speak.py            # Script Python TTS
  tts_speak_wrapper.sh    # Wrapper activant le venv (appelé par Rust)

~/.local/share/voice-speak/
  venv/                   # Environnement virtuel Python
  models/                 # Modèles de voix Piper (.onnx)
```

## Démarrage automatique à la connexion

Créez `~/.config/autostart/voice-speak.desktop` :

```ini
[Desktop Entry]
Type=Application
Name=voice-speak
Exec=voice-speak
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
```

## Dépannage

**Aucun son ne sort**
→ Le son passe par `paplay` (PulseAudio/PipeWire). Vérifiez votre sortie par défaut :
→ `pactl info | grep "Default Sink"`
→ ALSA brut (`sounddevice` direct) est contourné intentionnellement — il achemine silencieusement vers le mauvais périphérique sur de nombreux ordinateurs portables.

**`xclip: command not found`**
→ `sudo apt install xclip`

**`ERROR: No Piper model found`**
→ Les fichiers de modèle sont absents de `~/.local/share/voice-speak/models/`. Relancez `./install.sh` ou téléchargez manuellement (voir **Voix** ci-dessus).

**rdev nécessite des droits élevés**
→ `sudo voice-speak` (temporaire), ou ajoutez-vous au groupe `input` :
→ `sudo usermod -aG input $USER` puis déconnectez-vous et reconnectez-vous.

**Wayland / fenêtres purement Wayland**
→ Le presse-papier X11 est lu via `xclip`. Sous Wayland, le binaire détecte automatiquement `WAYLAND_DISPLAY` / `XDG_SESSION_TYPE` et utilise `wl-paste` à la place.
