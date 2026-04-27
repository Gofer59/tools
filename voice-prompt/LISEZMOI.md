# voice-prompt

> Parole-à-texte par pression de touche, saisie dans n'importe quelle fenêtre. Qualité Whisper, démarrage à chaud en 250 ms, paramètres modifiables en direct depuis une vraie interface graphique.

Maintenez une touche, parlez, relâchez — vos mots apparaissent à l'endroit du curseur. Le daemon Whisper se charge une seule fois au démarrage et reste en mémoire, de sorte que chaque transcription suivante est rapide. Tous les paramètres se changent en direct depuis une interface à trois onglets, sans redémarrage ni édition de fichier texte.

## Fonctionnalités

- Touche de dictée entièrement configurable
- Application immédiate des paramètres — les modifications prennent effet en moins d'une seconde, sans redémarrage
- Catalogue de 9 modèles Whisper (tiny à large-v3) avec téléchargement en continu
- Import de modèles `.onnx` personnalisés
- Daemon Python persistant — aucune latence de démarrage après le premier enregistrement
- Multiplateforme : Linux X11 et Windows 10/11
- Licence MIT, aucune télémétrie

## Plateformes prises en charge

| Plateforme | Statut |
|---|---|
| Linux X11 | Pris en charge |
| Linux Wayland | Partiel — les raccourcis globaux passent par le fallback rdev evdev ; l'utilisateur doit appartenir au groupe `input` |
| Windows 10/11 | Pris en charge |
| macOS | Non testé |

## Installation rapide

**Linux :**

```bash
git clone https://github.com/SVaiva/voice-tools
cd voice-tools/voice-prompt
cargo tauri build
./install.sh
```

**Windows (PowerShell) :**

```powershell
git clone https://github.com/SVaiva/voice-tools
cd voice-tools/voice-prompt
.\install.ps1 -FromSource
```

## Utilisation

Lancez `voice-prompt`. La fenêtre comporte trois onglets :

- **Settings** — configurez la touche de dictée, le modèle Whisper, la langue de transcription, le VAD, le chemin vers l'interpréteur Python et le type de calcul. Chaque modification est enregistrée sur disque et appliquée immédiatement ; le daemon redémarre en arrière-plan en environ une seconde.
- **Models** — parcourez le catalogue des neuf modèles Whisper (tiny, base, small, medium, large-v1/v2/v3, distil-large-v2, distil-large-v3), téléchargez-les avec une barre de progression en temps réel, ou importez un modèle `.onnx` personnalisé depuis le disque.
- **About** — numéro de version, licence, et sélecteur de langue pour l'interface elle-même.

Utilisation quotidienne : lancez `voice-prompt` une seule fois, configurez la touche et le modèle à votre convenance, puis réduisez dans la barre système. Ensuite, maintenez la touche pour enregistrer, relâchez pour transcrire — le texte est saisi à l'emplacement du curseur.

## Référence des paramètres

| Champ | Valeur par défaut | Effet |
|---|---|---|
| `push_to_talk_key` | `Ctrl+Alt+Space` | Maintenir pour enregistrer, relâcher pour transcrire et saisir |
| `whisper_model` | `small` | Taille du modèle Whisper utilisé pour la transcription |
| `language` | `en` | Langue de transcription (`en`, `fr`, `auto`) |
| `vad_filter` | `true` | Voice Activity Detection — supprime les silences en début et en fin |
| `python_bin` | `python3` | Chemin vers l'interpréteur Python qui exécute le daemon |
| `compute_type` | `int8` | Quantification Whisper (`int8`, `float16`, `float32`) |

La configuration est stockée en JSON :

- **Linux :** `~/.local/share/voice-prompt/config.json`
- **Windows :** `%LOCALAPPDATA%\voice-prompt\config.json`

## Modèles

Les modèles du catalogue sont téléchargés à la demande dans :

- **Linux :** `~/.local/share/voice-prompt/models/`
- **Windows :** `%LOCALAPPDATA%\voice-prompt\models\`

**Modèles personnalisés :** cliquez sur « Add custom model » dans l'onglet Models et sélectionnez votre fichier `.onnx`. Un fichier de configuration `.onnx.json` doit être présent dans le même répertoire — c'est le format standard de configuration `faster-whisper`.

Tailles de référence des modèles :

| Modèle | Taille sur disque | Vitesse relative |
|---|---|---|
| tiny | ~75 Mo | le plus rapide |
| base | ~145 Mo | rapide |
| small | ~465 Mo | bon compromis (défaut) |
| medium | ~1,5 Go | précis |
| large-v3 | ~3,1 Go | le plus précis |

## Dépannage

**Le raccourci ne fonctionne pas sous Wayland :**
Les raccourcis globaux sont enregistrés via tauri-plugin-global-shortcut. Sur Wayland, le mécanisme bascule sur le fallback rdev evdev. Vérifiez que votre utilisateur appartient au groupe `input` :

```bash
sudo gpasswd -a $USER input
```

Déconnectez-vous puis reconnectez-vous pour que le changement de groupe prenne effet.

**Aucun audio du microphone :**
Vérifiez les sources disponibles avec `pactl list sources short`. En cas d'utilisation directe d'ALSA, contrôlez la compatibilité du taux d'échantillonnage. Utilisateurs de PipeWire : assurez-vous que `pipewire-alsa` est installé.

**L'environnement virtuel Python est introuvable :**
Relancez `./install.sh`. Ou configurez-le manuellement :

```bash
python3 -m venv ~/.local/share/voice-prompt/venv
~/.local/share/voice-prompt/venv/bin/pip install faster-whisper
```

**Le téléchargement du modèle échoue :**
HuggingFace est peut-être temporairement indisponible. Attendez quelques minutes et réessayez. Vérifiez que votre réseau peut atteindre `huggingface.co`.

**Windows : l'antivirus bloque le MSI non signé :**
Ajoutez l'installateur à la liste d'autorisation, ou compilez depuis les sources avec l'indicateur `-FromSource` décrit ci-dessus.

**Windows : VC++ Redistributable ou WebView2 manquant :**
L'installateur gère WebView2 automatiquement. Pour le VC++ Redistributable, téléchargez-le depuis le site officiel de Microsoft.

**`libxdo` introuvable (erreur de compilation Linux) :**
Installez les en-têtes de développement :

```bash
# Debian / Ubuntu
sudo apt install libxdo-dev

# Fedora / RHEL
sudo dnf install libxdo-devel
```

`install.sh` gère cela automatiquement sur les distributions prises en charge.

## Compilation depuis les sources

Prérequis : Rust (via rustup), Node.js 20+ et `cargo-tauri`.

```bash
git clone https://github.com/SVaiva/voice-tools
cd voice-tools/voice-prompt

# Installer les dépendances de l'interface
cd ui && npm install && cd ..

# Lancer en mode développement (rechargement à chaud)
cd src-tauri && cargo tauri dev

# Compiler un binaire de publication
cargo tauri build
```

Le bundle de publication se trouve sous `src-tauri/target/release/bundle/`.

## Contribuer

Petit projet personnel — les PR sont les bienvenues. Veuillez exécuter `cargo fmt && cargo clippy` avant de soumettre. Ouvrez d'abord une issue pour les modifications importantes afin de valider la direction avant d'investir du temps.

## Licence

MIT — voir [LICENSE](../LICENSE).
