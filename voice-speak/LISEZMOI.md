# voice-speak

> Sélectionnez du texte, appuyez sur votre raccourci clavier, écoutez-le prononcé à voix haute. 23 voix Piper, 12 familles de langues, entièrement hors ligne, configurable en direct depuis une vraie interface graphique.

## Fonctionnalités

- TTS déclenché par raccourci clavier — lit le texte sélectionné à voix haute (sélection PRIMARY → CLIPBOARD en repli sur Linux)
- 23 voix Piper couvrant 12 langues : anglais (US/GB), français, allemand, espagnol, italien, chinois, portugais, russe, néerlandais, polonais, suédois, turc, ukrainien
- Application instantanée des réglages — voix, vitesse, paramètres de bruit prennent effet immédiatement, sans redémarrage
- Import de modèles `.onnx` personnalisés — ajoutez n'importe quelle voix compatible Piper depuis l'onglet Modèles
- Démon Python persistant — Piper se charge une seule fois au démarrage ; aucun délai entre les utilisations
- Arrêt de la lecture en rappuyant sur le raccourci (bascule)
- Entièrement hors ligne — aucune connexion internet requise après le téléchargement des modèles
- Multi-plateforme : Linux X11 + Windows 10/11
- Licence MIT, aucune télémétrie

## Plateformes prises en charge

| Plateforme | État |
|---|---|
| Linux X11 | ✅ Pris en charge |
| Linux Wayland | ⚠️ Partiel — raccourcis globaux via le repli evdev de rdev |
| Windows 10/11 | ✅ Pris en charge |
| macOS | ❌ Non testé |

## Installation rapide

**Linux :**
```bash
git clone https://github.com/SVaiva/voice-tools
cd voice-tools/voice-speak
cargo tauri build
./install.sh
```

**Windows :**
```powershell
git clone https://github.com/SVaiva/voice-tools
cd voice-tools/voice-speak
.\install.ps1 -FromSource
```

## Utilisation

- **Onglet Paramètres** : configurez le raccourci clavier, la voix, le curseur de vitesse, les paramètres de bruit et le chemin Python. Tous les changements s'appliquent immédiatement.
- **Onglet Modèles** : parcourez le catalogue de 23 voix organisé par langue, téléchargez des voix, ajoutez des voix `.onnx` personnalisées.
- **Onglet À propos** : version, licence, sélecteur de langue.

Au quotidien : lancez `voice-speak`, configurez une fois, réduisez dans la barre des tâches. Sélectionnez du texte dans n'importe quelle application, appuyez sur le raccourci pour l'entendre lire à voix haute. Appuyez à nouveau pour arrêter.

## Référence des paramètres

| Champ | Défaut | Effet |
|---|---|---|
| `hotkey` | `Ctrl+Alt+V` | Raccourci clavier pour démarrer/arrêter la synthèse vocale |
| `voice` | `en_US-lessac-medium` | Identifiant du modèle de voix Piper |
| `speed` | `1.0` | Vitesse de lecture (0.5–2.0) |
| `noise_scale` | `0.667` | Variation phonémique (0.0–1.0) |
| `noise_w_scale` | `0.8` | Variation de durée (0.0–1.5) |
| `python_bin` | `python3` | Chemin vers l'interpréteur Python |

La configuration est stockée en JSON dans `~/.local/share/voice-speak/config.json` sous Linux et `%LOCALAPPDATA%\voice-speak\config.json` sous Windows.

## Modèles

Les voix Piper se téléchargent dans `~/.local/share/voice-speak/models/` sous Linux, `%LOCALAPPDATA%\voice-speak\models\` sous Windows. Chaque voix est composée d'un fichier `.onnx` et d'un fichier de configuration `.onnx.json` associé.

Voix personnalisées : cliquez sur **Ajouter un modèle personnalisé** dans l'onglet Modèles — les deux fichiers `.onnx` et `.onnx.json` sont requis.

## Dépannage

**Le raccourci clavier ne fonctionne pas sous Wayland :**
rdev nécessite l'accès aux fichiers de périphériques `/dev/input`. Ajoutez votre utilisateur au groupe `input` :
```bash
sudo gpasswd -a $USER input
# puis déconnectez-vous et reconnectez-vous
```

**Pas de sortie audio :**
Vérifiez les sorties disponibles : `pactl list sinks short`. Assurez-vous que rodio peut ouvrir la sortie par défaut. Si vous utilisez PipeWire, vérifiez que `pipewire-pulse` est en cours d'exécution.

**xclip / wl-paste introuvable :**
Installez via le gestionnaire de paquets :
```bash
sudo apt install xclip wl-clipboard
```
`install.sh` gère cela automatiquement.

**Le téléchargement d'une voix échoue :**
HuggingFace peut être temporairement indisponible. Réessayez après un moment. Assurez-vous que votre réseau peut atteindre `huggingface.co`.

**Voix personnalisée sans .onnx.json :**
Piper requiert à la fois le fichier modèle `.onnx` ET son fichier de configuration `.onnx.json` associé. Téléchargez les deux fichiers depuis le [dépôt des voix Piper](https://github.com/rhasspy/piper/blob/master/VOICES.md).

**Environnement virtuel Python introuvable :**
Relancez `./install.sh`. Ou manuellement :
```bash
python3 -m venv ~/.local/share/voice-speak/venv
~/.local/share/voice-speak/venv/bin/pip install piper-tts numpy
```

## Compilation depuis les sources

```bash
git clone https://github.com/SVaiva/voice-tools
cd voice-tools/voice-speak
cd ui && npm install && cd ..
cd src-tauri && cargo tauri dev
# ou pour une version de production :
cargo tauri build
```

## Contribuer

Les contributions sont bienvenues. Exécutez `cargo fmt && cargo clippy` avant de soumettre une pull request.

## Licence

MIT — voir le fichier [LICENSE](../LICENSE).
