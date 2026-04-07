# Voice Speak (SteamDeck)

Synthese vocale pour le texte surligne sur SteamDeck. Appuyez sur un raccourci pour entendre n'importe quel texte selectionne lu a voix haute par vos haut-parleurs ; appuyez a nouveau pour arreter immediatement. Utilise Piper TTS (voix neurales ONNX, entierement hors ligne) et achemine l'audio via PulseAudio/PipeWire avec paplay. Lit la selection PRIMARY Wayland (texte surligne) automatiquement -- pas besoin de Ctrl+C. Se replie sur CLIPBOARD si PRIMARY est vide.

## Plateforme

SteamDeck (SteamOS 3.x / KDE Plasma / Wayland)

Fonctionne egalement sur les bureaux Linux standards (Debian, Ubuntu, Arch) sous Wayland.

## Prerequis

- **Rust 1.70+** et **Cargo** (pour la compilation, ou compilation croisee sur une autre machine)
- **Python 3** (pre-installe sur SteamOS)
- **wl-clipboard** (fournit `wl-paste` pour lire le texte surligne sous Wayland)
- **paplay** (pre-installe sur SteamOS en tant que partie de la compatibilite PipeWire)

> **Systeme de fichiers SteamOS en lecture seule :** `wl-clipboard` doit etre installe via pacman, ce qui necessite de deverrouiller temporairement le systeme de fichiers :
> ```bash
> sudo steamos-readonly disable
> sudo pacman -S wl-clipboard
> sudo steamos-readonly enable
> ```

Les dependances Python (`piper-tts`, `sounddevice`, `numpy`) sont installees automatiquement dans un venv isole par `install.sh`.

## Installation

```bash
cd voice-speak
chmod +x install.sh
./install.sh
```

L'installateur va :
1. Verifier les dependances systeme (python3, wl-paste)
2. Verifier l'appartenance au groupe `input` (requis pour les raccourcis rdev)
3. Creer un venv Python a `~/.local/share/voice-speak/venv/`
4. Installer les dependances Python (piper-tts, sounddevice, numpy)
5. Telecharger le modele vocal Piper par defaut `en_US-lessac-medium` (~60 Mo) vers `~/.local/share/voice-speak/models/`
6. Compiler le binaire Rust (ou utiliser un binaire pre-compile sur SteamOS)
7. Installer le tout dans `~/.local/bin/`

Assurez-vous que `~/.local/bin` est dans votre PATH :

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Compilation croisee pour SteamDeck

Si SteamOS manque d'en-tetes de developpement pour la compilation, compilez sur n'importe quelle machine Linux x86_64 :

```bash
cargo build --release
scp target/release/voice-speak deck@steamdeck:~/voice-speak/target/release/
```

Puis lancez `./install.sh` sur le SteamDeck -- il detecte le binaire pre-compile et ignore `cargo build`.

Alternativement, utilisez distrobox (livre avec SteamOS) :

```bash
distrobox create --name archlinux --image archlinux:latest
distrobox enter archlinux
sudo pacman -S rust base-devel alsa-lib
cd ~/voice-speak && cargo build --release
exit
```

## Utilisation

```bash
voice-speak          # commence a ecouter ; Ctrl-C pour quitter
```

| Action | Touche |
|--------|--------|
| **Lire le texte surligne** | Appuyez sur **Ctrl Droit** |
| **Arreter la lecture** | Appuyez a nouveau sur **Ctrl Droit** pendant la lecture |

1. Selectionnez / surlignez du texte dans n'importe quelle application
2. Appuyez sur **Ctrl Droit** -- le texte est lu a voix haute (~0,5--1 s de latence)
3. Pour arreter en cours de lecture : appuyez a nouveau sur **Ctrl Droit**

La recuperation du texte utilise `wl-paste --primary` (le texte actuellement surligne, pas besoin de Ctrl+C). Si PRIMARY est vide, se replie sur `wl-paste` (CLIPBOARD).

## Configuration

Toute la configuration se fait en modifiant `src/main.rs` dans la structure `Config`, puis en recompilant avec `./install.sh`.

### Raccourci clavier

```rust
hotkey: Key::ControlRight,  // Ctrl Droit (par defaut)
hotkey: Key::AltGr,         // Alt Droit
hotkey: Key::F10,           // F10
hotkey: Key::F13,           // utile pour le mappage des palettes arriere du Steam Deck
```

Noms de touches disponibles : https://docs.rs/rdev/latest/rdev/enum.Key.html

### Voix

```rust
voice: "en_US-lessac-medium".into(),    // masculin neutre (par defaut)
voice: "en_US-ryan-medium".into(),      // masculin expressif
voice: "en_GB-alan-medium".into(),      // masculin britannique
voice: "fr_FR-siwis-medium".into(),     // feminin francais
```

Le modele doit etre telecharge au prealable (voir **Telecharger une nouvelle voix** ci-dessous).

### Vitesse

```rust
speed: 1.0,    // normale (par defaut)
speed: 1.5,    // 50 % plus rapide
speed: 0.8,    // 20 % plus lent
```

### Telecharger une nouvelle voix

Chaque voix necessite deux fichiers : `.onnx` (modele) et `.onnx.json` (config). Telechargez depuis le [depot HuggingFace de Piper](https://huggingface.co/rhasspy/piper-voices/tree/v1.0.0) :

```bash
BASE="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/ryan/medium"
MODEL_DIR="$HOME/.local/share/voice-speak/models"

curl -L -o "$MODEL_DIR/en_US-ryan-medium.onnx"      "$BASE/en_US-ryan-medium.onnx"
curl -L -o "$MODEL_DIR/en_US-ryan-medium.onnx.json" "$BASE/en_US-ryan-medium.onnx.json"
```

Exemple de voix francaise :

```bash
BASE="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/fr/fr_FR/siwis/medium"
curl -L -o "$MODEL_DIR/fr_FR-siwis-medium.onnx"      "$BASE/fr_FR-siwis-medium.onnx"
curl -L -o "$MODEL_DIR/fr_FR-siwis-medium.onnx.json" "$BASE/fr_FR-siwis-medium.onnx.json"
```

Puis definissez `voice: "fr_FR-siwis-medium".into()` dans `src/main.rs` et recompilez.

### Parametres de synthese avances

Modifiez `SynthesisConfig` dans `tts_speak.py` (pas besoin de recompiler) :

```python
syn_config = SynthesisConfig(
    length_scale=1.0 / speed,   # duree des phonemes ; <1 = plus rapide, >1 = plus lent
    noise_scale=0.667,           # expressivite (0 = robotique, 1 = varie)
    noise_w_scale=0.8,           # variation du rythme (0 = plat, 1 = naturel)
)
```

## Architecture

```
voice-speak/
├── src/main.rs           Binaire Rust : boucle de touches, lecture du presse-papier (wl-paste), gestion des sous-processus
├── tts_speak.py          Python : synthese Piper TTS + lecture paplay
├── Cargo.toml            Dependances Rust (rdev, anyhow, ctrlc, libc)
├── requirements.txt      Dependances Python (piper-tts, sounddevice, numpy)
└── install.sh            Script de compilation + installation (venv, telechargement modele, support compilation croisee)
```

**Modele de threading :**
- **Thread principal** : machine a etats (Idle / Speaking) + gestion des sous-processus
- **Thread ecouteur rdev** : capture les evenements bruts du clavier, les envoie via un canal mpsc

**Isolation des sous-processus :** Les processus TTS sont lances avec `setsid()` dans leur propre groupe de processus. L'arret de la lecture envoie `SIGKILL` au PID negatif, tuant instantanement le groupe entier (shell, Python, paplay).

**Fichiers installes :**

| Chemin | Role |
|--------|------|
| `~/.local/bin/voice-speak` | Binaire compile |
| `~/.local/bin/tts_speak.py` | Script Python TTS |
| `~/.local/bin/tts_speak_wrapper.sh` | Wrapper venv-aware (auto-genere) |
| `~/.local/share/voice-speak/venv/` | Environnement virtuel Python |
| `~/.local/share/voice-speak/models/` | Modeles vocaux ONNX Piper |

## Notes SteamOS

- Les paquets installes via pacman (`wl-clipboard`) ne survivent pas aux mises a jour majeures de SteamOS -- relancez les commandes pacman apres les mises a jour
- L'utilisateur doit etre dans le groupe `input` pour les raccourcis rdev : `sudo usermod -aG input $USER` (puis redemarrez)
- Le systeme de fichiers est en lecture seule par defaut ; `install.sh` documente le deverrouillage/reverrouillage via `steamos-readonly`
- Tout ce qui se trouve dans `~/.local/` (binaire, venv, modeles) et `~/.rustup/` survit aux mises a jour
- L'appartenance au groupe `input` survit aux mises a jour
- Le seul composant fragile est `wl-clipboard` installe via pacman

## Limitations connues

- Les applications Electron (Discord, VS Code) ne renseignent pas la selection PRIMARY Wayland -- utilisez Ctrl+C d'abord, puis appuyez sur le raccourci
- Le Mode Jeu (Gamescope) peut ne pas transmettre les evenements clavier a rdev -- utilisez le Mode Bureau
- Pas de fichier de config -- tous les reglages necessitent de modifier `src/main.rs` et de recompiler
- Le binaire peut necessiter une compilation croisee si SteamOS manque d'en-tetes de developpement ALSA
- La sortie audio utilise la sortie PulseAudio/PipeWire par defaut du systeme
- Les pauses Piper sont controlees en interne par la ponctuation -- il n'y a pas de parametre de pause dedie

## Licence

MIT -- voir [LICENSE](../../LICENSE)
