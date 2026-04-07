# media-compress

Scripts CLI et aide-mémoire pour compresser des fichiers vidéo, image et audio sous Linux Mint.
Réduit considérablement la taille des fichiers tout en maintenant une qualité acceptable pour l'archivage personnel.

---

## Installation

```bash
sudo apt install ffmpeg jpegoptim optipng webp libavif-bin imagemagick
```

| Paquet | Fournit |
|--------|---------|
| `ffmpeg` | Encodage/conversion vidéo & audio |
| `jpegoptim` | Optimisation JPEG |
| `optipng` | Compression PNG sans perte |
| `webp` | `cwebp` / `dwebp` pour la conversion WebP |
| `libavif-bin` | `avifenc` / `avifdec` pour la conversion AVIF |
| `imagemagick` | `mogrify` pour le redimensionnement/conversion par lots |

---

## Scripts

Deux scripts prêts à l'emploi sont inclus : `compress-videos` et `compress-images`.

### Installer les scripts

```bash
cd media-compress
chmod +x compress-videos compress-images
cp compress-videos compress-images ~/.local/bin/
```

### compress-videos

Compresse un fichier vidéo unique ou toutes les vidéos d'un dossier en utilisant H.265 (HEVC). La sortie est placée dans un sous-dossier `compressed/` à côté de l'entrée.

```
compress-videos <fichier-ou-dossier> [crf]
```

| Argument | Par défaut | Description |
|----------|-----------|-------------|
| `fichier-ou-dossier` | `.` (répertoire courant) | Un fichier vidéo unique ou un répertoire de vidéos |
| `crf` | `28` | Niveau de qualité (plus bas = meilleure qualité, fichier plus grand) |

**Exemples :**

```bash
# Compresser une seule vidéo
compress-videos ~/Videos/enregistrement.mp4

# Compresser toutes les vidéos d'un dossier (bonne qualité, CRF 28)
compress-videos ~/Videos/vacances/

# Compression agressive (CRF 32, fichier nettement plus petit)
compress-videos ~/Videos/anciens-enregistrements/ 32

# Fichier unique avec CRF agressif
compress-videos ~/Videos/capture-ecran.mkv 32

# Haute qualité (CRF 24, fichiers plus grands)
compress-videos ~/Videos/important/ 24
```

Formats d'entrée supportés : `.mp4`, `.mkv`, `.avi`, `.mov`, `.webm`, `.wmv`

Sortie : sous-dossier `compressed/` à côté de l'entrée (tout converti en MP4/H.265)

Affiche un résumé de réduction de taille par fichier.

---

### compress-images

Optimise une image unique ou tous les fichiers JPEG/PNG d'un dossier sur place. Sauvegarde les originaux dans un sous-dossier `originals/`.

```
compress-images <fichier-ou-dossier> [qualité]
```

| Argument | Par défaut | Description |
|----------|-----------|-------------|
| `fichier-ou-dossier` | `.` (répertoire courant) | Un fichier image unique ou un répertoire d'images |
| `qualité` | `80` | Cible de qualité JPEG (1-100). PNG est toujours sans perte. |

**Exemples :**

```bash
# Compresser une seule image
compress-images ~/Pictures/photo.jpg

# Image unique avec qualité agressive
compress-images ~/Pictures/photo.jpg 60

# Optimiser toutes les images d'un dossier (qualité JPEG 80, PNG sans perte)
compress-images ~/Pictures/photos/

# Compression JPEG plus agressive (qualité 70)
compress-images ~/Pictures/captures/ 70

# Compression légère, garder haute qualité (qualité 90)
compress-images ~/Pictures/portfolio/ 90
```

Formats d'entrée supportés : `.jpg`, `.jpeg`, `.png`

Les originaux sont toujours sauvegardés avant modification.

---

## Compression vidéo (ffmpeg)

### Codecs

| Codec | Option | Avantages | Inconvénients |
|-------|--------|-----------|---------------|
| **H.265 (HEVC)** | `-c:v libx265` | Meilleur équilibre taille/qualité/vitesse | Certains anciens appareils ne peuvent pas le lire |
| **H.264** | `-c:v libx264` | Fonctionne partout | ~40 % plus grand que H.265 à qualité égale |
| **AV1** | `-c:v libsvtav1` | Fichiers les plus petits possibles | 2-3x plus lent à encoder |

### Commandes

**Compression rapide** (H.265, recommandée par défaut) :
```bash
ffmpeg -i input.mp4 -c:v libx265 -crf 28 -preset medium -c:a aac -b:a 128k output.mp4
```

**Compression agressive** (fichier plus petit, réduction à 720p) :
```bash
ffmpeg -i input.mp4 -c:v libx265 -crf 32 -preset slow -vf scale=-2:720 -c:a aac -b:a 96k output.mp4
```

**Compatible** (H.264, fonctionne partout) :
```bash
ffmpeg -i input.mp4 -c:v libx264 -crf 23 -preset medium -c:a aac -b:a 128k output.mp4
```

**Compression maximale** (AV1, lent mais le plus petit) :
```bash
ffmpeg -i input.mp4 -c:v libsvtav1 -crf 35 -preset 6 -c:a libopus -b:a 128k output.mkv
```

**Par lots** (toutes les vidéos du répertoire courant) :
```bash
mkdir -p compressed
for f in *.mp4 *.mkv *.avi *.mov; do
  [ -f "$f" ] || continue
  ffmpeg -i "$f" -c:v libx265 -crf 28 -preset medium -c:a aac -b:a 128k "compressed/${f%.*}.mp4"
done
```

### Paramètres clés

| Paramètre | Rôle | Valeurs |
|-----------|------|---------|
| `-crf` | Qualité. Plus bas = meilleure qualité, fichier plus grand. | H.265 : **24** (haute) / **28** (équilibré) / **32** (agressif). H.264 : **18** / **23** / **28**. AV1 : **30** / **35** / **40**. |
| `-preset` | Vitesse vs compression. Plus lent = fichier plus petit à qualité égale. | `ultrafast` / `fast` / **`medium`** / `slow` / `veryslow`. Par défaut **medium**. |
| `-vf scale=-2:720` | Réduction de résolution. `-2` préserve le ratio d'aspect. | `720` (agressif), `1080` (garder la HD complète), `480` (très petit). |
| `-c:a aac -b:a 128k` | Codec audio et débit. | `128k` (musique), `96k` (voix), `64k` (minimum). |

### Guide visuel CRF (H.265)

| CRF | Cas d'utilisation | Réduction typique |
|-----|------------------|-------------------|
| 22-24 | Archivage, conserver qualité proche de l'original | 40-60 % |
| 26-28 | Usage général, visuellement identique pour la plupart | 60-80 % |
| 30-32 | Anciens enregistrements, captures d'écran, moins important | 75-90 % |
| 34+ | Aperçus, vignettes, qualité non importante | 85-95 % |

### Vérifier les tailles de fichiers

```bash
ls -lh input.mp4 output.mp4         # fichier unique
du -sh original/ compressed/         # comparer les dossiers
```

---

## Compression d'images

### JPEG

**Avec perte** (réduire à la qualité cible, supprimer les métadonnées) :
```bash
jpegoptim --max=80 --strip-all photo.jpg
```

**Sans perte** (optimiser l'encodage, supprimer les métadonnées, zéro perte de qualité) :
```bash
jpegoptim --strip-all photo.jpg
```

**Par lots** (tous les JPEG du répertoire courant) :
```bash
jpegoptim --max=80 --strip-all *.jpg *.jpeg
```

| Valeur `--max` | Cas d'utilisation |
|----------------|------------------|
| 90-95 | Photographie, garder haute qualité |
| 80-85 | Photos générales, visuellement identiques |
| 60-70 | Vignettes web, aperçus |

### PNG

**Optimisation sans perte** (recompresser avec de meilleurs algorithmes) :
```bash
optipng -o5 screenshot.png
```

| Niveau `-o` | Vitesse | Compression |
|-------------|---------|-------------|
| `-o2` | Rapide | Modérée |
| **`-o5`** | Équilibrée | Bonne |
| `-o7` | Lente | Maximale |

**Par lots :**
```bash
optipng -o5 *.png
```

### Convertir en WebP

25-35 % plus petit que JPEG à qualité équivalente :
```bash
cwebp -q 80 photo.jpg -o photo.webp
```

**Conversion par lots :**
```bash
for f in *.jpg *.jpeg *.png; do
  [ -f "$f" ] || continue
  cwebp -q 80 "$f" -o "${f%.*}.webp"
done
```

| Valeur `-q` | Cas d'utilisation |
|-------------|------------------|
| 85-90 | Haute qualité |
| **75-80** | Équilibré |
| 50-60 | Agressif, artéfacts visibles |

### Convertir en AVIF

Meilleur ratio de compression de tous les formats d'image :
```bash
avifenc --min 20 --max 30 photo.jpg photo.avif
```

Note : tous les lecteurs/applications ne supportent pas encore l'AVIF. Idéal pour l'archivage en conservant l'original.

### Redimensionner (ImageMagick)

Réduire à une largeur maximale (conserve le ratio d'aspect, ignore les images déjà plus petites) :
```bash
mogrify -resize '1920x>' *.jpg       # max 1920px de large
mogrify -resize '1280x>' *.png       # max 1280px de large
```

---

## Compression audio

### WAV / FLAC vers Opus (recommandé)

Opus offre le meilleur rapport qualité/débit de tous les codecs audio :
```bash
ffmpeg -i enregistrement.wav -c:a libopus -b:a 128k enregistrement.opus
```

| Débit | Cas d'utilisation |
|-------|------------------|
| 64k | Discours, podcasts, mémos vocaux |
| **128k** | Musique, usage général (qualité transparente) |
| 192k | Audiophile, écoute critique |

### WAV / FLAC vers AAC (compatibilité étendue)

```bash
ffmpeg -i enregistrement.wav -c:a aac -b:a 192k enregistrement.m4a
```

### Ré-encoder MP3 à un débit inférieur

```bash
ffmpeg -i volumineux.mp3 -c:a libmp3lame -b:a 128k plus-petit.mp3
```

Note : ré-encoder un format avec perte perd toujours un peu de qualité. Ne le faites que quand le gain de taille le justifie.

### Convertir par lots tous les WAV/FLAC en Opus

```bash
for f in *.wav *.flac; do
  [ -f "$f" ] || continue
  ffmpeg -i "$f" -c:a libopus -b:a 128k "${f%.*}.opus"
done
```

---

## Référence rapide

### Commandes vidéo en une ligne
```bash
ffmpeg -i in.mp4 -c:v libx265 -crf 28 -preset medium -c:a aac -b:a 128k out.mp4   # équilibré
ffmpeg -i in.mp4 -c:v libx265 -crf 32 -preset slow -vf scale=-2:720 -c:a aac -b:a 96k out.mp4  # agressif
ffmpeg -i in.mp4 -c:v libx264 -crf 23 -preset medium -c:a aac -b:a 128k out.mp4   # compatible
ffmpeg -i in.mp4 -c:v libsvtav1 -crf 35 -preset 6 -c:a libopus -b:a 128k out.mkv  # le plus petit
```

### Commandes image en une ligne
```bash
jpegoptim --max=80 --strip-all *.jpg        # JPEG avec perte
jpegoptim --strip-all *.jpg                  # JPEG sans perte
optipng -o5 *.png                            # PNG sans perte
cwebp -q 80 in.jpg -o out.webp              # JPEG/PNG vers WebP
mogrify -resize '1920x>' *.jpg               # réduction de taille
```

### Commandes audio en une ligne
```bash
ffmpeg -i in.wav -c:a libopus -b:a 128k out.opus     # meilleur qualité/taille
ffmpeg -i in.wav -c:a aac -b:a 192k out.m4a          # compatible
ffmpeg -i in.mp3 -c:a libmp3lame -b:a 128k out.mp3   # ré-encoder MP3
```
