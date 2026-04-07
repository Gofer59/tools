# media-compress

CLI scripts and cheat sheet for compressing video, image, and audio files on Linux Mint.
Reduces file sizes significantly while maintaining acceptable quality for personal archival.

---

## Installation

```bash
sudo apt install ffmpeg jpegoptim optipng webp libavif-bin imagemagick
```

| Package | Provides |
|---------|----------|
| `ffmpeg` | Video & audio encoding/conversion |
| `jpegoptim` | JPEG optimization |
| `optipng` | PNG lossless compression |
| `webp` | `cwebp` / `dwebp` for WebP conversion |
| `libavif-bin` | `avifenc` / `avifdec` for AVIF conversion |
| `imagemagick` | `mogrify` for batch resize/conversion |

---

## Scripts

Two ready-to-use scripts are included: `compress-videos` and `compress-images`.

### Install scripts

```bash
cd media-compress
chmod +x compress-videos compress-images
cp compress-videos compress-images ~/.local/bin/
```

### compress-videos

Compresses a single video file or all videos in a folder using H.265 (HEVC). Outputs to a `compressed/` subfolder next to the input.

```
compress-videos <file-or-folder> [crf]
```

| Argument | Default | Description |
|----------|---------|-------------|
| `file-or-folder` | `.` (current dir) | A single video file or a directory of videos |
| `crf` | `28` | Quality level (lower = better quality, bigger file) |

**Examples:**

```bash
# Compress a single video
compress-videos ~/Videos/recording.mp4

# Compress all videos in a folder (good quality, CRF 28)
compress-videos ~/Videos/vacation/

# Aggressive compression (CRF 32, noticeably smaller)
compress-videos ~/Videos/old-recordings/ 32

# Single file with aggressive CRF
compress-videos ~/Videos/screencast.mkv 32

# High quality (CRF 24, larger files)
compress-videos ~/Videos/important/ 24
```

Supported input formats: `.mp4`, `.mkv`, `.avi`, `.mov`, `.webm`, `.wmv`

Output: `compressed/` subfolder next to the input (all converted to MP4/H.265)

Prints per-file size reduction summary.

---

### compress-images

Optimizes a single image or all JPEG/PNG files in a folder in-place. Backs up originals to an `originals/` subfolder.

```
compress-images <file-or-folder> [quality]
```

| Argument | Default | Description |
|----------|---------|-------------|
| `file-or-folder` | `.` (current dir) | A single image file or a directory of images |
| `quality` | `80` | JPEG quality target (1-100). PNG is always lossless. |

**Examples:**

```bash
# Compress a single image
compress-images ~/Pictures/photo.jpg

# Single image with aggressive quality
compress-images ~/Pictures/photo.jpg 60

# Optimize all images in a folder (JPEG quality 80, PNG lossless)
compress-images ~/Pictures/photos/

# More aggressive JPEG compression (quality 70)
compress-images ~/Pictures/screenshots/ 70

# Light compression, keep high quality (quality 90)
compress-images ~/Pictures/portfolio/ 90
```

Supported input formats: `.jpg`, `.jpeg`, `.png`

Originals are always backed up before modification.

---

## Video Compression (ffmpeg)

### Codecs

| Codec | Flag | Pros | Cons |
|-------|------|------|------|
| **H.265 (HEVC)** | `-c:v libx265` | Best size/quality/speed balance | Some old devices can't play it |
| **H.264** | `-c:v libx264` | Plays everywhere | ~40% larger than H.265 at same quality |
| **AV1** | `-c:v libsvtav1` | Smallest files possible | 2-3x slower to encode |

### Commands

**Quick compress** (H.265, recommended default):
```bash
ffmpeg -i input.mp4 -c:v libx265 -crf 28 -preset medium -c:a aac -b:a 128k output.mp4
```

**Aggressive compress** (smaller file, downscale to 720p):
```bash
ffmpeg -i input.mp4 -c:v libx265 -crf 32 -preset slow -vf scale=-2:720 -c:a aac -b:a 96k output.mp4
```

**Compatible** (H.264, plays on everything):
```bash
ffmpeg -i input.mp4 -c:v libx264 -crf 23 -preset medium -c:a aac -b:a 128k output.mp4
```

**Maximum compression** (AV1, slow but smallest):
```bash
ffmpeg -i input.mp4 -c:v libsvtav1 -crf 35 -preset 6 -c:a libopus -b:a 128k output.mkv
```

**Batch** (all videos in current directory):
```bash
mkdir -p compressed
for f in *.mp4 *.mkv *.avi *.mov; do
  [ -f "$f" ] || continue
  ffmpeg -i "$f" -c:v libx265 -crf 28 -preset medium -c:a aac -b:a 128k "compressed/${f%.*}.mp4"
done
```

### Key Parameters

| Parameter | What it does | Values |
|-----------|-------------|--------|
| `-crf` | Quality. Lower = better quality, larger file. | H.265: **24** (high) / **28** (balanced) / **32** (aggressive). H.264: **18** / **23** / **28**. AV1: **30** / **35** / **40**. |
| `-preset` | Speed vs compression. Slower = smaller file at same quality. | `ultrafast` / `fast` / **`medium`** / `slow` / `veryslow`. Default to **medium**. |
| `-vf scale=-2:720` | Downscale resolution. `-2` preserves aspect ratio. | `720` (aggressive), `1080` (keep full HD), `480` (very small). |
| `-c:a aac -b:a 128k` | Audio codec and bitrate. | `128k` (music), `96k` (voice), `64k` (minimum). |

### CRF visual guide (H.265)

| CRF | Use case | Typical reduction |
|-----|----------|-------------------|
| 22-24 | Archival, keep near-original quality | 40-60% |
| 26-28 | General use, visually identical to most eyes | 60-80% |
| 30-32 | Old recordings, screen captures, less important | 75-90% |
| 34+ | Previews, thumbnails, quality not important | 85-95% |

### Check file sizes

```bash
ls -lh input.mp4 output.mp4         # single file
du -sh original/ compressed/         # compare folders
```

---

## Image Compression

### JPEG

**Lossy** (reduce to target quality, strips metadata):
```bash
jpegoptim --max=80 --strip-all photo.jpg
```

**Lossless** (optimize encoding, strip metadata, zero quality loss):
```bash
jpegoptim --strip-all photo.jpg
```

**Batch** (all JPEGs in current directory):
```bash
jpegoptim --max=80 --strip-all *.jpg *.jpeg
```

| `--max` value | Use case |
|---------------|----------|
| 90-95 | Photography, keep high quality |
| 80-85 | General photos, visually identical |
| 60-70 | Web thumbnails, previews |

### PNG

**Lossless optimization** (recompress with better algorithms):
```bash
optipng -o5 screenshot.png
```

| `-o` level | Speed | Compression |
|------------|-------|-------------|
| `-o2` | Fast | Moderate |
| **`-o5`** | Balanced | Good |
| `-o7` | Slow | Maximum |

**Batch:**
```bash
optipng -o5 *.png
```

### Convert to WebP

25-35% smaller than JPEG at equivalent quality:
```bash
cwebp -q 80 photo.jpg -o photo.webp
```

**Batch convert:**
```bash
for f in *.jpg *.jpeg *.png; do
  [ -f "$f" ] || continue
  cwebp -q 80 "$f" -o "${f%.*}.webp"
done
```

| `-q` value | Use case |
|------------|----------|
| 85-90 | High quality |
| **75-80** | Balanced |
| 50-60 | Aggressive, noticeable artifacts |

### Convert to AVIF

Best compression ratio of any image format:
```bash
avifenc --min 20 --max 30 photo.jpg photo.avif
```

Note: not all viewers/apps support AVIF yet. Best for archival where you keep the original.

### Resize (ImageMagick)

Downscale to max width (keeps aspect ratio, skips images already smaller):
```bash
mogrify -resize '1920x>' *.jpg       # max 1920px wide
mogrify -resize '1280x>' *.png       # max 1280px wide
```

---

## Audio Compression

### WAV / FLAC to Opus (recommended)

Opus offers the best quality-per-bit of any audio codec:
```bash
ffmpeg -i recording.wav -c:a libopus -b:a 128k recording.opus
```

| Bitrate | Use case |
|---------|----------|
| 64k | Speech, podcasts, voice memos |
| **128k** | Music, general purpose (transparent quality) |
| 192k | Audiophile, critical listening |

### WAV / FLAC to AAC (wider device compatibility)

```bash
ffmpeg -i recording.wav -c:a aac -b:a 192k recording.m4a
```

### Re-encode MP3 to lower bitrate

```bash
ffmpeg -i bloated.mp3 -c:a libmp3lame -b:a 128k smaller.mp3
```

Note: re-encoding a lossy format always loses some quality. Only do this when the size savings are worth it.

### Batch convert all WAV/FLAC to Opus

```bash
for f in *.wav *.flac; do
  [ -f "$f" ] || continue
  ffmpeg -i "$f" -c:a libopus -b:a 128k "${f%.*}.opus"
done
```

---

## Quick Reference

### Video one-liners
```bash
ffmpeg -i in.mp4 -c:v libx265 -crf 28 -preset medium -c:a aac -b:a 128k out.mp4   # balanced
ffmpeg -i in.mp4 -c:v libx265 -crf 32 -preset slow -vf scale=-2:720 -c:a aac -b:a 96k out.mp4  # aggressive
ffmpeg -i in.mp4 -c:v libx264 -crf 23 -preset medium -c:a aac -b:a 128k out.mp4   # compatible
ffmpeg -i in.mp4 -c:v libsvtav1 -crf 35 -preset 6 -c:a libopus -b:a 128k out.mkv  # smallest
```

### Image one-liners
```bash
jpegoptim --max=80 --strip-all *.jpg        # JPEG lossy
jpegoptim --strip-all *.jpg                  # JPEG lossless
optipng -o5 *.png                            # PNG lossless
cwebp -q 80 in.jpg -o out.webp              # JPEG/PNG to WebP
mogrify -resize '1920x>' *.jpg               # downscale
```

### Audio one-liners
```bash
ffmpeg -i in.wav -c:a libopus -b:a 128k out.opus     # best quality/size
ffmpeg -i in.wav -c:a aac -b:a 192k out.m4a          # compatible
ffmpeg -i in.mp3 -c:a libmp3lame -b:a 128k out.mp3   # re-encode MP3
```
