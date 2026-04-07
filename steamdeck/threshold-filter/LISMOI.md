# Threshold Filter (SteamDeck)

Filtre de seuil d'ecran en temps reel pour SteamDeck. Capture une region d'ecran via grim/slurp, applique un seuil de luminance BT.601 (conversion en noir et blanc pur), et affiche le resultat dans une fenetre overlay sans bordure et toujours au premier plan via egui. Concu pour ameliorer la lisibilite du texte dans les romans visuels et les jeux -- utile comme etape de pre-traitement OCR ou pour lire du texte a faible contraste. Rafraichit automatiquement la region capturee a une frequence d'images configurable. Supporte a la fois X11 (via xcap/slop) et Wayland (via grim/slurp).

## Plateforme

SteamDeck (SteamOS 3.x / KDE Plasma / Wayland)

Fonctionne egalement sur les bureaux Linux standards (Arch, etc.) sous X11 ou Wayland.

## Prerequis

- **Rust 1.82+** et **Cargo** (pour la compilation)
- Paquets systeme (installes automatiquement par `install.sh`) :
  - `grim` (capture d'ecran Wayland)
  - `slurp` (selection interactive de region Wayland)
  - `maim`, `slop` (capture et selection X11 en repli)
  - `xdotool` (gestion de fenetres X11)

> **Systeme de fichiers SteamOS en lecture seule :** L'installateur deverrouille temporairement le systeme de fichiers avec `steamos-readonly disable`, installe les paquets via pacman, puis le reverrouille. Le systeme de fichiers est egalement reverrouille automatiquement en cas d'echec de l'installateur.

## Installation

### Option A : Compiler sur le SteamDeck (si cargo est disponible)

```bash
cd threshold-filter
chmod +x install.sh
./install.sh
```

L'installateur compilera le binaire si cargo est disponible et qu'aucun binaire pre-compile n'existe.

### Option B : Compilation croisee sur votre machine de developpement

Compilez sur n'importe quel PC Linux x86_64 avec Rust 1.82+ :

```bash
cd threshold-filter
cargo build --release
```

Copiez le dossier sur le SteamDeck et lancez l'installateur :

```bash
scp -r threshold-filter/ deck@steamdeck:~/threshold-filter/
# Sur le SteamDeck :
cd ~/threshold-filter
chmod +x install.sh
./install.sh
```

L'installateur va :
1. Deverrouiller le systeme de fichiers SteamOS et installer `grim`, `slurp` via pacman
2. Reverrouiller le systeme de fichiers (reverrouille aussi automatiquement en cas d'echec)
3. Verifier/proposer d'ajouter votre utilisateur au groupe `input`
4. Installer le binaire dans `~/.local/bin/threshold-filter-deck`
5. Creer une entree dans le menu d'applications KDE

Assurez-vous que `~/.local/bin` est dans votre PATH :

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## Utilisation

```bash
threshold-filter-deck    # lance la fenetre overlay
```

Ou cherchez "Threshold Filter" dans le menu d'applications KDE.

| Raccourci | Action |
|-----------|--------|
| `F10` | **Selectionner une region** -- dessinez un rectangle avec slurp (Wayland) ou cliquez sur une fenetre + dessinez avec slop (X11) |
| `F8` | **Basculer toujours au premier plan** -- garder l'overlay au-dessus des autres fenetres |

### Controles de l'interface

La fenetre overlay possede un panneau gauche repliable avec :

- **Curseur de seuil** (0--255) : faites glisser pour ajuster le seuil noir/blanc
- **Bouton Sel** : selectionner une nouvelle region d'ecran (identique a F10)
- **Bouton Cap** : capturer la region actuelle
- **Bascule Inv** : inverser la sortie (echanger noir et blanc)
- **Bouton Quit** : fermer l'application

Le panneau droit affiche l'image seuillee, en preservant le rapport d'aspect exact de la region capturee. L'image se rafraichit automatiquement a une frequence d'images configurable.

## Configuration

Fichier de config : `~/.config/threshold-filter/config.toml`

Cree automatiquement avec les valeurs par defaut au premier lancement. Modifiez avec n'importe quel editeur de texte ; redemarrez pour que les changements prennent effet.

```toml
[hotkeys]
region_select   = "F10"       # Selectionner une nouvelle region d'ecran
toggle_on_top   = "F8"        # Basculer toujours au premier plan

[display]
default_threshold = 128       # Seuil de luminance 0-255
invert            = false     # Echanger la sortie noir/blanc
always_on_top     = true      # L'overlay reste au-dessus des autres fenetres
panel_width       = 50.0      # Largeur du panneau de controle gauche en pixels
```

### Format des raccourcis

- Touches nommees : `F8`, `F9`, `F10`, `MetaLeft`, `KeyQ`, etc.
- Combinaisons : `"MetaLeft+KeyQ"`, `"AltLeft+KeyU"`
- Codes de touches bruts depuis Steam Input : `"191"` ou `"Unknown(191)"`

## Architecture

```
threshold-filter/
├── src/
│   ├── main.rs          Orchestrateur 3 threads : ecouteur rdev, dispatcher de raccourcis, boucle principale egui
│   ├── capture.rs       Wrappers grim/slurp + slop/maim, structure Region, persistance JSON
│   ├── processing.rs    Seuil de luminance BT.601 (niveaux de gris -> noir/blanc binaire)
│   ├── config.rs        Chargement de config TOML avec creation automatique et valeurs par defaut
│   └── ui.rs            egui : panneau gauche repliable, curseur de seuil, affichage d'image avec preservation du rapport d'aspect
├── Cargo.toml           Dependances Rust (eframe, egui, xcap, rdev, image, anyhow, toml, serde, ...)
└── install.sh           Installateur SteamDeck (pacman, groupe input, entree menu)
```

**Modele de threading :**
1. **Ecouteur rdev** (thread en arriere-plan) -- capture les evenements bruts du clavier depuis `/dev/input`
2. **Dispatcher de raccourcis** (thread en arriere-plan) -- fait correspondre les combinaisons de touches, lance slurp/slop, envoie des actions via des canaux
3. **Boucle principale egui** (thread principal) -- interroge les canaux avec `try_recv`, capture l'ecran via grim/xcap, applique le seuil, rend l'interface

**Detection auto du serveur d'affichage :**
- **Wayland :** `slurp` (selection de region) + `grim` (capture)
- **X11 :** `xdotool` (selection de fenetre) + `slop` (selection de region) + `xcap` (capture de fenetre)

**Fichiers installes :**

| Chemin | Role |
|--------|------|
| `~/.local/bin/threshold-filter-deck` | Binaire compile |
| `~/.config/threshold-filter/config.toml` | Configuration (cree au premier lancement) |
| `~/.local/share/threshold-filter/last_region.json` | Region de capture persistee |
| `~/.local/share/applications/threshold-filter-deck.desktop` | Entree menu d'applications KDE |

## Notes SteamOS

- Les paquets installes via pacman (`grim`, `slurp`) ne survivent pas aux mises a jour majeures de SteamOS -- relancez `install.sh` apres les mises a jour
- L'utilisateur doit etre dans le groupe `input` pour les raccourcis rdev : `sudo usermod -aG input $USER` (puis redemarrez)
- Le systeme de fichiers est en lecture seule par defaut ; `install.sh` gere le deverrouillage/reverrouillage via `steamos-readonly`
- Tout ce qui se trouve dans `~/.local/` et `~/.config/` (binaire, config, fichier de region) survit aux mises a jour
- L'appartenance au groupe `input` survit aux mises a jour
- L'installateur reverrouille le systeme de fichiers automatiquement, meme si une etape echoue (via trap)

## Limitations connues

- Le Mode Jeu (Gamescope) peut ne pas transmettre les evenements clavier a rdev -- utilisez le Mode Bureau
- Le filtre de seuil est purement visuel (noir/blanc binaire) -- il n'effectue pas d'OCR lui-meme
- Sous Wayland, la capture d'ecran utilise `grim` qui capture la sortie entiere puis recadre ; il peut y avoir un bref scintillement a chaque capture
- Le panneau gauche se replie pour economiser l'espace mais necessite une hauteur de fenetre minimale pour afficher tous les controles
- Pas d'apercu en direct pendant la selection de region (slurp/slop gerent cela independamment)
- Necessite Rust 1.82+ en raison des exigences de version eframe/egui

## Licence

MIT -- voir [LICENSE](../../LICENSE)
