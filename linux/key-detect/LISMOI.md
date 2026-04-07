# key-detect

Utilitaire minimal de detection de codes de touches. Appuyez sur n'importe quelle touche pour voir son nom `rdev::Key` ou son code brut. Utile pour trouver les noms de touches corrects a utiliser dans les fichiers de configuration des autres outils (threshold-filter, deck-reader, screen-ocr, etc.).

## Plateforme

Linux (X11). Necessite le groupe `input` pour la detection globale des touches.

## Prerequis

```bash
# Chaine d'outils Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# L'utilisateur doit etre dans le groupe input (pour rdev)
sudo usermod -aG input $USER
# Puis redemarrer
```

## Installation

```bash
cd key-detect
cargo build --release
cp target/release/key-detect ~/.local/bin/
```

## Utilisation

```bash
key-detect
```

Appuyez sur n'importe quelle touche. La sortie affiche :

```
Key detection utility — press any key (Ctrl+C to quit)
Look for Key::Unknown(N) values to use in your rdev Config.

KeyPress    F9
KeyRelease  F9
KeyPress    Key::Unknown(191)   <- use Key::Unknown(191) in Config
KeyRelease  Key::Unknown(191)   <- use Key::Unknown(191) in Config
```

- Les touches nommees (F1-F12, Escape, etc.) affichent leur nom de variante `rdev::Key`
- Les touches non nommees affichent `Key::Unknown(N)` — utilisez ce code exact dans la configuration de vos outils
- Appuyez sur **Ctrl+C** pour quitter

## Architecture

```
src/
  main.rs    Utilitaire mono-fichier : ecouteur rdev qui affiche les evenements clavier
```

## Limitations connues

- Necessite le groupe `input` sous Linux (rdev accede a `/dev/input/*`)
- Ne detecte que les evenements clavier, pas la souris ni les manettes

## Licence

MIT — voir [LICENSE](../../LICENSE)
