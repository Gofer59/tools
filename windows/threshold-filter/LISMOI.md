# threshold-filter

Filtre de seuillage en temps reel pour le preprocessing OCR sous Windows. Selectionnez une fenetre d'application, tracez un rectangle sur la zone de texte, et obtenez une vue seuillage noir et blanc en temps reel. Les fonds sombres deviennent noirs, le texte clair devient blanc -- ideal pour ameliorer la precision de l'OCR sur du texte a faible contraste ou bruite.

Aucun outil externe necessaire : la selection de fenetre et le tracage de region sont integres a l'application.

## Plateforme

Windows 10/11

## Prerequis

- Toolchain Rust ([https://rustup.rs](https://rustup.rs))

## Installation

```powershell
cargo build --release
```

L'executable se trouve dans `target\release\threshold-filter.exe`. Copiez-le ou vous voulez :

```powershell
mkdir %USERPROFILE%\.local\bin 2>nul
copy target\release\threshold-filter.exe %USERPROFILE%\.local\bin\
```

## Utilisation

Lancez `threshold-filter.exe` en double-cliquant dessus ou depuis un terminal :

```powershell
threshold-filter.exe
```

### Fonctionnement

1. Au premier lancement, une liste des fenetres ouvertes s'affiche -- cliquez sur celle que vous voulez capturer
2. Le contenu complet de la fenetre apparait en apercu -- **tracez un rectangle** sur la zone de texte a seuiller
3. L'application se redimensionne pour correspondre a la region selectionnee et se positionne par-dessus
4. Une vue seuillee en temps reel se met a jour a ~15 FPS
5. Ajustez le curseur **Thr** pour affiner le seuil noir/blanc (0--255)
6. Cochez **Inv** pour inverser le noir et le blanc
7. Cliquez sur **Sel** (ou appuyez sur **F10**) pour choisir une autre fenetre et zone
8. Cliquez sur **Quit** pour fermer

### Disposition de l'interface

Les controles sont dans un panneau vertical etroit a gauche. L'image seuillee remplit l'espace restant a droite. Le panneau est defilable si la fenetre est trop courte pour afficher tous les controles.

```
+------+-----------------------------+
| Thr  |                             |
| |==| |    Image seuillee           |
| |  | |    (remplit l'espace        |
| |==| |     restant)                |
|      |                             |
| Sel  |                             |
| Inv  |                             |
| Top  |                             |
| Move |                             |
| < >  |                             |
| /\ \/|                             |
| Quit |                             |
+------+-----------------------------+
```

### Controles

| Controle | Description |
|----------|-------------|
| **Thr** (curseur 0--255) | Seuil de luminosite. Les pixels plus clairs que cette valeur deviennent blancs ; les plus sombres deviennent noirs. Par defaut : 128 |
| Bouton **Sel** | Choisir une nouvelle fenetre et zone. Equivalent a appuyer sur F10 |
| Case **Inv** | Inverser noir et blanc |
| Case **Top** | Garder la fenetre au-dessus de toutes les autres |
| Boutons **Move** (`< > /\ \/`) | Deplacer la fenetre overlay de 20 pixels par clic |
| Bouton **Quit** | Fermer l'application |

### Raccourcis clavier globaux

| Touche | Action |
|--------|--------|
| **F10** | Re-selectionner fenetre et zone |
| **F9** | Activer/desactiver le mode toujours au premier plan |

Les raccourcis fonctionnent globalement -- ils sont detectes meme quand une autre fenetre est active (par exemple un jeu), y compris les jeux avec logiciel anti-triche comme Genshin Impact. Les raccourcis utilisent Win32 `RegisterHotKey` (pas un hook bas niveau), ils ne peuvent donc pas etre bloques par l'anti-triche. Configurables via le fichier de configuration.

## Configuration

Les parametres sont charges depuis un fichier de configuration TOML :

```
%APPDATA%\threshold-filter\config.toml
```

Un fichier de configuration par defaut est cree automatiquement au premier lancement. Modifiez-le avec n'importe quel editeur de texte, puis relancez l'application pour appliquer les changements.

### Configuration par defaut

```toml
[hotkeys]
# Noms de touches : F1-F12, Escape, Tab, Space, A-Z, etc.
# Combinaisons avec modificateur : MetaLeft+KeyQ, AltLeft+KeyU, ControlLeft+KeyR
# Codes de touches bruts : "191" ou "Unknown(191)"
region_select   = "F10"
toggle_on_top   = "F9"

[display]
default_threshold = 128       # 0-255
invert            = false     # inverser noir/blanc
always_on_top     = true
```

### Noms de touches supportes

`F1`--`F12`, `Escape`/`Esc`, `Tab`, `Enter`/`Return`, `Space`, `Backspace`, `Delete`, `Home`, `End`, `A`--`Z` (ou `KeyA`--`KeyZ`), fleches directionnelles (`UpArrow`, `DownArrow`, `LeftArrow`, `RightArrow`), modificateurs (`MetaLeft`, `AltLeft`, `ControlLeft`, `ShiftLeft`, et leurs variantes `Right`), et codes de touches bruts (`191` ou `Unknown(191)`).

Les combinaisons avec modificateur utilisent `+` comme separateur : `ControlLeft+F5`, `AltLeft+KeyU`.

## Raccourci bureau

1. Appuyez sur **Win + R**, tapez `shell:desktop`, appuyez sur Entree
2. Clic droit sur le bureau, selectionnez **Nouveau > Raccourci**
3. Pour l'emplacement, entrez le chemin complet vers `threshold-filter.exe` (par exemple `C:\Users\VotreNom\.local\bin\threshold-filter.exe`)
4. Nommez-le `Threshold Filter`
5. (Optionnel) Clic droit sur le raccourci, selectionnez **Proprietes > Changer d'icone** pour choisir une icone personnalisee

## Architecture

```
src/
  main.rs          Point d'entree, hotkeys globaux (Win32 RegisterHotKey), configuration eframe
  config.rs        Configuration TOML : hotkeys, parametres d'affichage, resolution du chemin %APPDATA%
  capture.rs       Capture par fenetre via xcap (backend WGC sous Windows)
  processing.rs    Seuillage binaire (luminance BT.601, arithmetique en virgule fixe)
  ui.rs            Application egui : selecteur de fenetre integre, tracage de region par
                   drag, panneau lateral defilable, toggle toujours au premier plan, deplacement
```

L'outil utilise `xcap` avec Windows Graphics Capture (WGC) pour capturer uniquement les pixels de la fenetre selectionnee -- pas de fond de bureau, pas d'autres fenetres, pas d'auto-capture. L'image capturee est recadree sur la sous-region tracee par l'utilisateur, puis un seuillage binaire par pixel est applique et affiche via egui a ~15 FPS.

La mise a l'echelle DPI est geree automatiquement : le traceur de region convertit entre les coordonnees physiques (image capturee) et logiques (ecran) en utilisant le rapport entre la taille de l'image capturee et la taille logique rapportee par la fenetre.

### Dependances (crates)

| Crate | Role |
|-------|------|
| [eframe](https://crates.io/crates/eframe) / egui | Framework d'interface graphique |
| [xcap](https://crates.io/crates/xcap) 0.8 (WGC) | Capture d'ecran par fenetre, y compris les jeux DirectX/Vulkan |
| [image](https://crates.io/crates/image) | Recadrage d'image |
| [serde](https://crates.io/crates/serde) + [toml](https://crates.io/crates/toml) | Lecture du fichier de configuration |
| [rdev](https://crates.io/crates/rdev) | Listener de raccourcis clavier globaux |
| [raw-window-handle](https://crates.io/crates/raw-window-handle) | Extraction de l'identifiant natif de fenetre (pour filtrer l'auto-capture) |

## Limitations connues

- **La fenetre cible doit rester ouverte.** Si la fenetre capturee est fermee ou minimisee, la capture echoue silencieusement et l'affichage se fige sur la derniere image.
- **Le pointeur de souris peut apparaitre dans les captures.** `xcap` demande a Windows Graphics Capture d'exclure le curseur (`SetIsCursorCaptureEnabled(false)`), mais sur certaines versions de Windows ce reglage n'est pas respecte de maniere fiable pour les captures par fenetre. Si vous voyez votre pointeur clignoter dans l'image seuillee, eloignez-le de la fenetre cible ou figez la capture en mettant l'application cible en pause.
- **Pas de support multi-moniteur avec DPI differents.** La mise a l'echelle DPI est calculee a partir de la fenetre capturee ; si l'overlay et la cible sont sur des ecrans avec des DPI differents, l'alignement peut etre legerement decale.
- **La taille du binaire est d'environ 18 Mo** a cause du backend de rendu egui.
- **Certaines fenetres peuvent ne pas apparaitre dans la liste** si elles rapportent une taille nulle ou n'ont pas de titre.
- **Les applications UWP/Store** peuvent necessiter de lancer l'outil en tant qu'Administrateur pour que la capture WGC fonctionne.

## Licence

MIT -- voir [LICENSE](../../LICENSE)
