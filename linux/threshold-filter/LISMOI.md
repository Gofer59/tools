# threshold-filter

Filtre de seuil en temps reel pour le pretraitement OCR. Cliquez sur une fenetre d'application, puis dessinez un rectangle pour selectionner la zone de texte. L'outil capture uniquement le contenu de cette fenetre (pas le bureau ni d'autres fenetres) et affiche un seuil binaire en temps reel. Les arriere-plans sombres deviennent noirs, le texte clair devient blanc.

Fonctionne sur **Linux (X11)** et **Windows**.

## Compilation

Necessite la chaine d'outils Rust (`cargo`).

### Linux

```bash
# Dependances de compilation (Ubuntu/Debian)
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libxkbcommon-dev libgl-dev pkg-config

# Dependances d'execution
sudo apt-get install xdotool slop

# Compiler et installer
./install.sh
```

### Windows

```powershell
cargo build --release
copy target\release\threshold-filter.exe %USERPROFILE%\.local\bin\
```

Aucun outil systeme supplementaire n'est necessaire — la selection de fenetre et le dessin de region sont integres a l'application sous Windows.

#### Raccourci bureau

1. Appuyez sur **Win + R**, tapez `shell:desktop`, appuyez sur Entree
2. Clic droit -> **Nouveau -> Raccourci**
3. Emplacement : `%USERPROFILE%\.local\bin\threshold-filter.exe`
4. Nom : `Threshold Filter`
5. (Optionnel) Clic droit sur le raccourci -> **Proprietes -> Changer d'icone**

## Utilisation

```
threshold-filter
```

### Linux (X11)

1. **Etape 1 :** Cliquez sur la fenetre a capturer (`xdotool selectwindow`)
2. **Etape 2 :** Dessinez un rectangle sur la zone a filtrer (`slop`)
3. La fenetre d'affichage se redimensionne et se positionne sur la zone selectionnee
4. Une vue seuillee en temps reel se met a jour a ~15 FPS
5. Appuyez sur **F10** pour selectionner une autre fenetre et zone

### Windows

1. Lancez `threshold-filter.exe`
2. Cliquez sur **Sel** dans le panneau gauche (ou appuyez sur **F10**)
3. Une liste des fenetres ouvertes apparait — cliquez sur celle a capturer
4. Le contenu de la fenetre s'affiche en apercu — **dessinez un rectangle** sur la zone de texte
5. L'application se redimensionne et se positionne sur la zone
6. Une vue seuillee en temps reel se met a jour a ~15 FPS
7. Ajustez le curseur **Thr** pour affiner le seuil noir/blanc
8. Cliquez a nouveau sur **Sel** (ou **F10**) pour changer de fenetre
9. Cliquez sur **Quit** pour fermer

### Disposition de l'interface

Les controles sont dans un panneau vertical etroit a gauche. L'image seuillee remplit l'espace restant a droite en conservant les proportions exactes.

```
+------+-----------------------------+
| Thr  |                             |
| |==| |    Image seuillee           |
| |  | |    (proportions exactes)    |
| |==| |                             |
|      |                             |
| Sel  |                             |
| Top  |                             |
| Move |                             |
| < >  |                             |
| /\ \/|                             |
| Quit |                             |
+------+-----------------------------+
 panneau      droite : l'image remplit
 gauche       l'espace restant
(defilable)
```

### Controles

- **Thr (curseur vertical 0-255) :** Ajuste le seuil de luminosite. Defaut : 128.
- **Bouton Sel :** Choisir une nouvelle fenetre et zone (defaut : F10)
- **Case Inv :** Inverser noir/blanc
- **Case Top :** Garder la fenetre au premier plan (defaut : F9 pour basculer)
- **Boutons Move (< > /\ \/) :** Deplacer la fenetre de 20px par clic
- **Bouton Quit :** Fermer l'application

### Raccourcis clavier (defaut)

| Touche | Action |
|--------|--------|
| F10 | Selectionner fenetre + zone |
| F9  | Basculer toujours au premier plan |

Les raccourcis fonctionnent globalement — ils sont detectes meme quand une autre fenetre est active (ex. un jeu). Configurables via le fichier de configuration (voir ci-dessous).

## Configuration

Les parametres sont charges depuis un fichier de configuration TOML :
- **Linux :** `~/.config/threshold-filter/config.toml`
- **Windows :** `%APPDATA%\threshold-filter\config.toml`

Un fichier de configuration par defaut est cree au premier lancement. Modifiez-le pour personnaliser les raccourcis et les parametres d'affichage, puis redemarrez l'application.

### Configuration par defaut

```toml
[hotkeys]
# Noms de touches : F1-F12, Escape, Tab, Space, A-Z, etc.
# Combinaisons : MetaLeft+KeyQ, AltLeft+KeyU, ControlLeft+KeyR
# Codes bruts : "191" ou "Unknown(191)"
region_select   = "F10"
toggle_on_top   = "F9"

[display]
default_threshold = 128       # 0-255
invert            = false     # inverser noir/blanc
always_on_top     = true
```

### Noms de touches

`F1`-`F12`, `Escape`/`Esc`, `Tab`, `Enter`/`Return`, `Space`, `Backspace`, `Delete`, `Home`, `End`, `A`-`Z` (ou `KeyA`-`KeyZ`), touches flechees (`UpArrow`, `DownArrow`, `LeftArrow`, `RightArrow`), modificateurs (`MetaLeft`, `AltLeft`, `ControlLeft`, etc.), et codes bruts (`191` ou `Unknown(191)`).

## Fonctionnement

L'outil utilise `xcap::Window::capture_image()` pour capturer uniquement les pixels de la fenetre selectionnee — pas d'arriere-plan de bureau, pas d'autres fenetres, pas d'auto-capture. L'image capturee est rognee a la sous-region dessinee par l'utilisateur, puis un seuil binaire par pixel (luminance BT.601 >= curseur -> blanc, sinon -> noir) est applique et affiche a ~15 FPS.

**Differences par plateforme :**
- **Linux :** La selection utilise des outils externes (`xdotool` + `slop`). Le deplacement utilise `xdotool windowmove`.
- **Windows :** La selection est entierement integree (liste de fenetres + rectangle a dessiner). Le deplacement utilise `ViewportCommand` d'egui.

## Architecture

```
src/
  main.rs          Point d'entree, chargement de la configuration, fenetre eframe
  config.rs        Fichier de configuration TOML : raccourcis, parametres d'affichage
  capture.rs       Capture par fenetre via xcap::Window + rognage de sous-region
  processing.rs    Filtre de seuil binaire (luminance BT.601, virgule fixe)
  ui.rs            Application egui : selection conditionnelle par plateforme, panneau gauche defilable
```

## Dependances d'execution

### Linux
- `xdotool` — selection de fenetre, requete de geometrie et deplacement
- `slop` — dessin interactif de rectangle de sous-region

### Windows
- Aucune — toute l'interface de selection est integree a l'application

## Limitations connues

- **Linux :** Necessite X11 (`xdotool` et `slop` ne fonctionnent pas sous Wayland). Pour SteamDeck/Wayland, utilisez la variante `steamdeck/`. Les raccourcis globaux necessitent le groupe `input` (`sudo usermod -aG input $USER`, puis redemarrer).
- **La fenetre cible doit rester ouverte :** si elle est fermee, la capture echoue proprement
- **Taille du binaire :** ~18 Mo a cause du backend de rendu egui

## Licence

MIT — voir [LICENSE](../../LICENSE)
