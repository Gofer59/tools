# screen-ocr

OCR de région d'écran déclenché par raccourci clavier avec synthèse vocale pour Linux. Conçu pour les romans visuels sur SteamDeck : sélectionnez la zone de dialogue une fois, puis appuyez sur une seule touche à chaque nouvelle ligne pour l'extraire, la copier dans le presse-papier et l'entendre lue à voix haute. Entièrement hors ligne — aucun service cloud, aucune clé d'API.

## Fonctionnement

```
F10  -->  Dessiner un rectangle de sélection (sauvegardé sur disque)
F9   -->  Re-capture instantanée de la région sauvegardée  -->  OCR Tesseract  -->  Presse-papier  -->  TTS
```

```
Binaire Rust (screen-ocr)
  |-- rdev             écouteur global de touches (F9 / F10)
  |-- slop / slurp     sélection interactive de région (X11 / Wayland)
  |-- maim / grim      capture de région d'écran (X11 / Wayland)
  |-- tempfile         fichier PNG temporaire avec nettoyage automatique
  |-- serde_json       sauvegarde la géométrie dans ~/.local/share/screen-ocr/
  |-- sous-processus --> python3 ocr_extract.py <image>
  |                       '-- pytesseract (OCR Tesseract, hors ligne, CPU)
  |-- xclip / wl-copy  livraison dans le presse-papier
  '-- sous-processus --> tts_speak_wrapper.sh <texte> <voix> <vitesse>
                           '-- Piper TTS (ONNX, CPU, hors ligne) --> paplay
```

**Flux de travail roman visuel :**
1. Appuyez sur **F10** pour dessiner un rectangle autour de la zone de texte de dialogue (une seule fois)
2. Appuyez sur **F9** à chaque avancement du dialogue — la région sauvegardée est re-capturée instantanément
3. Le texte est extrait, copié dans le presse-papier et lu à voix haute automatiquement
4. Appuyez à nouveau sur **F10** si la zone de texte se déplace ou si vous changez de jeu

La géométrie de la région sélectionnée est sauvegardée dans `~/.local/share/screen-ocr/last_region.json` et persiste entre les redémarrages.

---

## Installation

### Prérequis

- **Rust** (1.70+) — installez via [rustup](https://rustup.rs/) :
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Python 3.10+** — pré-installé sur la plupart des distributions Linux
- **voice-speak** (optionnel, pour la TTS) — installez depuis le projet `voice-speak/` voisin :
  ```bash
  cd ../voice-speak && ./install.sh
  ```

### Linux (Debian / Ubuntu / Linux Mint)

```bash
git clone <ce-dépôt> && cd screen-ocr
chmod +x install.sh
./install.sh
```

L'installateur gère tout automatiquement :

1. Détecte votre serveur d'affichage (X11 ou Wayland)
2. Installe les paquets système via `apt` :
   - **X11 :** `maim`, `slop`, `xclip`, `xdotool`, `tesseract-ocr`, `libasound2-dev`
   - **Wayland :** `grim`, `slurp`, `wl-clipboard`, `tesseract-ocr`, `libasound2-dev`
3. Vérifie la TTS voice-speak (optionnel)
4. Crée un environnement virtuel Python à `~/.local/share/screen-ocr/venv/`
5. Installe les dépendances Python : `pytesseract`, `Pillow`
6. Compile le binaire Rust en mode release
7. Installe tout dans `~/.local/bin/`

### SteamDeck (SteamOS)

SteamOS est basé sur Arch et utilise un système de fichiers racine immuable (lecture seule) par défaut. Vous devez le déverrouiller avant d'installer des paquets système.

#### Étape 1 : Déverrouiller le système de fichiers

```bash
sudo steamos-readonly disable
```

> **Note :** Les mises à jour de SteamOS peuvent réactiver le système de fichiers en lecture seule, vous obligeant à relancer cette commande après les mises à jour.

#### Étape 2 : Initialiser le trousseau pacman (première fois seulement)

```bash
sudo pacman-key --init
sudo pacman-key --populate archlinux
sudo pacman-key --populate holo
```

#### Étape 3 : Installer les dépendances système

Le Mode Bureau SteamDeck tourne sous KDE Plasma sur Wayland, vous avez donc besoin des outils Wayland :

```bash
sudo pacman -S --noconfirm tesseract tesseract-data-eng grim slurp wl-clipboard alsa-lib python
```

Pour les romans visuels japonais (ou autres langues) :

```bash
sudo pacman -S tesseract-data-jpn    # japonais
sudo pacman -S tesseract-data-chi_sim  # chinois simplifié
```

Si vous souhaitez également le mode de livraison `Type` (saisie du texte au curseur sous Wayland) :

```bash
sudo pacman -S --noconfirm ydotool
sudo systemctl enable --now ydotoold
```

#### Étape 4 : Installer Rust (si pas encore installé)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

#### Étape 5 : Installer voice-speak pour la TTS (recommandé)

```bash
cd voice-speak
chmod +x install.sh
./install.sh
```

#### Étape 6 : Compiler et installer screen-ocr

```bash
cd screen-ocr
chmod +x install.sh
./install.sh
```

L'installateur détecte `pacman` et s'adapte en conséquence.

#### Étape 7 : S'assurer que le PATH est configuré

Ajoutez à votre `~/.bashrc` si pas déjà présent :

```bash
export PATH="$HOME/.local/bin:$PATH"
```

#### Limitations SteamDeck

- **Mode Bureau** (KDE Plasma / Wayland) : entièrement supporté. Utilise `grim` + `slurp` pour la capture, `wl-copy` pour le presse-papier, Piper pour la TTS.
- **Mode Jeu** (compositeur Gamescope) : l'écouteur de touches global (`rdev`) peut ne pas recevoir les événements clavier à travers Gamescope. Utilisez le Mode Bureau pour de meilleurs résultats.

---

## Utilisation

```bash
screen-ocr          # tourne au premier plan, Ctrl-C pour quitter
```

1. Appuyez sur **F10** pour dessiner un rectangle de sélection autour de la zone de texte
2. La région est sauvegardée et l'OCR s'exécute immédiatement — le texte apparaît dans le presse-papier et est lu à voix haute
3. Appuyez sur **F9** pour re-capturer la même région instantanément (pas besoin de dessiner)
4. Appuyez à nouveau sur **F9** à chaque avancement du dialogue
5. Appuyez sur **F10** pour sélectionner une région différente à tout moment

Au démarrage, `screen-ocr` affiche sa configuration active :

```
+============================================+
|         screen-ocr  ready                   |
+============================================+
|  F9:       Quick capture (re-use region)    |
|  F10:      Select new region                |
|  Display:  X11                              |
|  Capture:  slop + maim -g                   |
|  Clipboard:xclip                            |
|  TTS:      Piper (voice-speak)              |
|  Region:   loaded from disk                 |
|                                             |
|  F10 -> draw region -> OCR -> clipboard     |
|  F9  -> instant re-capture -> OCR -> TTS    |
|  Ctrl-C to quit                             |
+============================================+
```

### Annuler une sélection

Appuyez sur **Échap** pendant que le réticule est visible (lors du F10) pour annuler et revenir à l'état d'attente.

### Démarrage automatique à la connexion

Créez `~/.config/autostart/screen-ocr.desktop` :

```ini
[Desktop Entry]
Type=Application
Name=screen-ocr
Exec=screen-ocr
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
```

---

## Configuration

Toute la configuration se fait en modifiant `src/main.rs` dans la structure `Config`, puis en recompilant avec `./install.sh`.

### Raccourcis clavier

Modifiez `quick_capture_key` et `select_region_key` dans `Config::default()` :

```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            quick_capture_key: Key::F9,     // re-capture la région mémorisée
            select_region_key: Key::F10,    // sélection interactive
            // ...
        }
    }
}
```

#### Touches disponibles

| Valeur | Touche physique |
|--------|----------------|
| `Key::F1` .. `Key::F12` | Touches de fonction |
| `Key::KeyA` .. `Key::KeyZ` | Touches A-Z |
| `Key::Num0` .. `Key::Num9` | Touches numériques |
| `Key::Space` | Barre espace |
| `Key::PrintScreen` | Impr. écran |
| `Key::MetaLeft` / `Key::MetaRight` | Touches Super/Meta/Windows |
| `Key::Alt` / `Key::AltGr` | Touches Alt |
| `Key::ControlLeft` / `Key::ControlRight` | Touches Ctrl |

#### Exemples

| Capture rapide | Sélection région | Configuration |
|----------------|-----------------|---------------|
| F9 | F10 | Par défaut |
| F7 | F8 | `quick_capture_key: Key::F7, select_region_key: Key::F8` |
| Début | Fin | `quick_capture_key: Key::Home, select_region_key: Key::End` |
| ImprÉcran | Pause | `quick_capture_key: Key::PrintScreen, select_region_key: Key::Pause` |

### Mode de livraison

Contrôle ce qui se passe avec le texte extrait. Modifiez `delivery_mode` dans `Config::default()` :

```rust
delivery_mode: DeliveryMode::Clipboard,
```

| Mode | Comportement | Outils utilisés |
|------|-------------|-----------------|
| `DeliveryMode::Clipboard` | Copie dans le presse-papier système (par défaut) | `xclip` (X11) / `wl-copy` (Wayland) |
| `DeliveryMode::Type` | Tape le texte à la position courante du curseur | `xdotool` (X11) / `ydotool` (Wayland) |
| `DeliveryMode::Both` | Copie ET tape au curseur | Les deux ci-dessus |

`Clipboard` est le mode par défaut car la sortie OCR nécessite souvent de légères corrections avant utilisation.

### TTS (synthèse vocale)

La TTS nécessite l'outil `voice-speak` installé (fournit Piper TTS + audio paplay). Si non installé, screen-ocr fonctionne normalement mais sans parole.

Modifiez les paramètres TTS dans `Config::default()` :

```rust
tts_wrapper: PathBuf::from(".../.local/bin/tts_speak_wrapper.sh"),
tts_voice: "en_US-lessac-medium".into(),  // modèle de voix Piper
tts_speed: "1.0".into(),                   // 1.0 = normal, 1.5 = plus rapide, 0.8 = plus lent
```

| Paramètre | Par défaut | Description |
|-----------|-----------|-------------|
| `tts_wrapper` | `~/.local/bin/tts_speak_wrapper.sh` | Chemin vers le wrapper TTS voice-speak |
| `tts_voice` | `en_US-lessac-medium` | Nom du modèle de voix Piper |
| `tts_speed` | `1.0` | Multiplicateur de vitesse (1.0 = normal) |

La TTS est non bloquante : la parole se joue en arrière-plan pendant que vous continuez à interagir avec le jeu. Si vous appuyez à nouveau sur F9 avant que la parole précédente soit terminée, elle est interrompue immédiatement et le nouveau texte commence.

### Persistance de la géométrie de région

La région sélectionnée est stockée en JSON à :

```
~/.local/share/screen-ocr/last_region.json
```

Format :
```json
{
  "x": 100,
  "y": 500,
  "w": 800,
  "h": 200
}
```

Ce fichier persiste entre les redémarrages. Supprimez-le pour forcer une nouvelle sélection lors du prochain appui sur F9.

### Langue Tesseract

Par défaut, Tesseract utilise l'anglais. Pour ajouter d'autres langues :

#### Installer les paquets de langue

```bash
# Debian/Ubuntu/Mint
sudo apt install tesseract-ocr-fra   # français
sudo apt install tesseract-ocr-deu   # allemand
sudo apt install tesseract-ocr-jpn   # japonais
sudo apt install tesseract-ocr-chi-sim  # chinois simplifié

# Arch/SteamOS
sudo pacman -S tesseract-data-fra
sudo pacman -S tesseract-data-deu
sudo pacman -S tesseract-data-jpn
sudo pacman -S tesseract-data-chi_sim
```

#### Configurer la langue dans le script Python

Modifiez `python/ocr_extract.py` et changez l'appel à `image_to_string` :

```python
# Langue unique
text = pytesseract.image_to_string(img, lang='fra').strip()

# Plusieurs langues (Tesseract essaie toutes, choisit la meilleure)
text = pytesseract.image_to_string(img, lang='eng+fra+deu').strip()

# Romans visuels japonais
text = pytesseract.image_to_string(img, lang='jpn').strip()
```

Puis relancez `./install.sh` pour déployer le script mis à jour.

#### Lister les langues installées

```bash
tesseract --list-langs
```

### Moteur OCR

Le moteur par défaut est Tesseract via `pytesseract`. Pour le remplacer par un autre moteur, modifiez `python/ocr_extract.py`. Le contrat est simple :

- **Entrée :** chemin de l'image comme `sys.argv[1]`
- **Sortie :** texte extrait affiché sur `stdout`
- **Diagnostic :** affichez sur `stderr`

Par exemple, pour utiliser EasyOCR à la place :

```python
import sys
import easyocr

reader = easyocr.Reader(['en'], gpu=False)
results = reader.readtext(sys.argv[1], detail=0)
print('\n'.join(results))
```

Ajoutez `easyocr` à `requirements.txt` et relancez `./install.sh`.

---

## Structure des fichiers

```
screen-ocr/
|-- src/main.rs              Binaire Rust (touches, capture, OCR, presse-papier, TTS)
|-- Cargo.toml               Dépendances Rust
|-- python/ocr_extract.py    Script Python OCR (wrapper Tesseract)
|-- requirements.txt         Dépendances Python (pytesseract, Pillow)
|-- install.sh               Installateur multi-distribution (apt / pacman)
'-- README.md / LISMOI.md    Documentation
```

Après installation :

```
~/.local/bin/
|-- screen-ocr               Binaire compilé
|-- ocr_extract.py            Script Python OCR
'-- ocr_extract_wrapper.sh    Wrapper venv-aware (auto-généré)

~/.local/share/screen-ocr/
|-- venv/                     Environnement virtuel Python
'-- last_region.json          Géométrie de région sauvegardée (créée au premier F10)
```

---

## Dépannage

### "Failed to run slop" / "Failed to run slurp"

L'outil de sélection de région pour votre serveur d'affichage n'est pas installé :

```bash
# X11
sudo apt install slop           # Debian/Ubuntu/Mint
sudo pacman -S slop             # Arch/SteamOS

# Wayland
sudo apt install slurp          # Debian/Ubuntu
sudo pacman -S slurp            # Arch/SteamOS
```

### "Failed to run maim" / "Failed to run grim"

L'outil de capture d'écran n'est pas installé :

```bash
# X11
sudo apt install maim           # Debian/Ubuntu/Mint
sudo pacman -S maim             # Arch/SteamOS

# Wayland
sudo apt install grim           # Debian/Ubuntu
sudo pacman -S grim             # Arch/SteamOS
```

### "No saved region" sur F9

Aucune région n'a encore été sélectionnée. Appuyez d'abord sur F10 pour dessiner un rectangle de sélection. La région est ensuite sauvegardée et F9 fonctionnera.

### "Failed to run OCR script" / "Did you run install.sh?"

Le venv Python ou le script wrapper est manquant. Relancez `./install.sh`.

### "TTS error (continuing without speech)"

Le wrapper TTS voice-speak n'est pas installé. Installez-le :

```bash
cd ../voice-speak && ./install.sh
```

screen-ocr continuera à fonctionner sans TTS — l'OCR et le presse-papier fonctionnent normalement.

### Sortie OCR vide ou illisible

- Assurez-vous d'avoir sélectionné une région avec du texte lisible (pas seulement des images/icônes).
- Tesseract fonctionne mieux sur du texte à fort contraste avec des polices standard.
- Pour du très petit texte, essayez de sélectionner une zone plus grande ou de zoomer d'abord.
- Vérifiez que le bon pack de langue est installé : `tesseract --list-langs`
- Pour le texte japonais, installez `tesseract-data-jpn` et définissez `lang='jpn'` dans le script Python.

### "rdev error" sous Wayland

Le crate `rdev` utilise X11 pour l'écoute des événements clavier. Sur Wayland pur, il nécessite XWayland. KDE Plasma sur le Mode Bureau SteamDeck exécute XWayland par défaut, donc cela devrait fonctionner sans configuration supplémentaire.

### SteamOS : "error: could not open file ... Permission denied"

Le système de fichiers est en lecture seule. Lancez :

```bash
sudo steamos-readonly disable
```

### xdotool / ydotool ne tape pas le texte

- **X11 :** Vérifiez que `xdotool` est installé : `sudo apt install xdotool`
- **Wayland (SteamDeck) :** Vérifiez que `ydotool` est installé et que son démon tourne :
  ```bash
  sudo pacman -S ydotool
  sudo systemctl enable --now ydotoold
  ```

---

## Dépendances

### Crates Rust

| Crate | Version | Rôle |
|-------|---------|------|
| `rdev` | 0.5 | Écouteur global d'événements clavier |
| `tempfile` | 3 | Fichiers temporaires avec nettoyage automatique |
| `anyhow` | 1 | Gestion ergonomique des erreurs |
| `ctrlc` | 3 | Arrêt gracieux sur Ctrl-C |
| `libc` | 0.2 | Gestion des groupes de processus (setsid, kill) pour la TTS |
| `serde` | 1 | Sérialisation de la structure de région |
| `serde_json` | 1 | Persistance JSON de la géométrie de région |

### Paquets système

| Paquet | X11 | Wayland | Rôle |
|--------|-----|---------|------|
| `maim` | Requis | — | Capture de région d'écran |
| `slop` | Requis | — | Sélection interactive de région |
| `grim` | — | Requis | Capture d'écran |
| `slurp` | — | Requis | Sélection interactive de région |
| `xclip` | Requis | — | Accès au presse-papier |
| `wl-clipboard` | — | Requis | Accès au presse-papier |
| `xdotool` | Optionnel | — | Saisie au curseur |
| `ydotool` | — | Optionnel | Saisie au curseur |
| `tesseract-ocr` | Requis | Requis | Moteur OCR |
| `libasound2-dev` / `alsa-lib` | Requis | Requis | En-têtes ALSA (dépendance de compilation pour rdev) |

### Paquets Python

| Paquet | Rôle |
|--------|------|
| `pytesseract` | Wrapper Python pour le CLI Tesseract |
| `Pillow` | Chargement d'images (PIL) |

### Optionnel : voice-speak (TTS)

| Composant | Emplacement | Rôle |
|-----------|------------|------|
| `tts_speak_wrapper.sh` | `~/.local/bin/` | Point d'entrée TTS venv-aware |
| `tts_speak.py` | `~/.local/bin/` | Script Python Piper TTS |
| Modèle Piper | `~/.local/share/voice-speak/models/` | Modèle de voix ONNX |
| Environnement virtuel Python | `~/.local/share/voice-speak/venv/` | Piper + dépendances |
