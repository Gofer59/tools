# gamebook-digitize

Convert a video of someone flipping through a physical gamebook into:

1. **Markdown source** — editable, with numbered `## § N` sections and image references
2. **Self-contained HTML player** — dark theme, sidebar with character sheet, section navigation, clickable cross-references

Designed for French "Livres dont vous êtes le héros" and English Fighting Fantasy / Choose Your Own Adventure books.

## Install

```bash
cd tools/gamebook-digitize
chmod +x install.sh && ./install.sh
```

This creates a Python venv at `~/.local/share/gamebook-digitize/venv/` and a launcher at `~/.local/bin/gamebook-digitize`.

**Surya OCR** (default engine) downloads ~1-2 GB of model weights on first run, cached in `~/.cache/huggingface/`.

**Tesseract** (optional fallback) requires separate installation:
```bash
# Debian/Ubuntu
sudo apt-get install -y tesseract-ocr tesseract-ocr-fra tesseract-ocr-eng

# Arch/SteamOS
sudo pacman -S tesseract tesseract-data-fra tesseract-data-eng
```

## Usage

### Full pipeline — video to gamebook

```bash
# French gamebook, 3 reference pages (cover, character sheet, equipment)
gamebook-digitize input.mp4 --lang fr --ref-pages 3

# English book, custom output directory
gamebook-digitize input.mp4 --lang en --ref-pages 5 --output ./my-book/

# Skip LLM cleanup (faster, no Claude CLI needed)
gamebook-digitize input.mp4 --lang fr --ref-pages 3 --no-llm

# Use Tesseract instead of Surya
gamebook-digitize input.mp4 --lang fr --ref-pages 3 --ocr-engine tesseract

# Tuning parameters
gamebook-digitize input.mp4 --lang fr --ref-pages 3 \
  --frame-interval 0.3 \
  --sharpness-threshold 80 \
  --hash-threshold 6

# Debug: keep extracted frames, verbose output
gamebook-digitize input.mp4 --lang fr --ref-pages 3 --keep-frames --verbose
```

### Regenerate HTML from edited markdown

After editing `sections.md` to fix OCR errors or adjust section text:

```bash
gamebook-digitize --from-markdown my-book/sections.md
```

This regenerates `player.html` without re-processing the video.

## Output

```
my-book/
├── sections.md    # Editable markdown source
├── player.html    # Self-contained HTML gamebook player
├── images/        # Extracted images
│   ├── ref_001.jpg        # Reference pages (sidebar)
│   ├── sec001_fig1.jpg    # Section illustrations (inline)
│   └── ...
└── frames/        # (only with --keep-frames) Selected page images
```

## Pipeline Stages

| Stage | Description |
|-------|-------------|
| 1 | **Frame extraction** — OpenCV extracts frames at configurable interval |
| 2 | **Best frame selection** — Laplacian sharpness + frame differencing picks one frame per stable page |
| 3 | **Page deduplication** — Perceptual hashing removes duplicates (consecutive + global) |
| 4 | **OCR** — Surya (default) or Tesseract extracts text with bounding boxes |
| 5 | **Section splitting** — Detects large section numbers (§ N) by font size, splits text into sections |
| 6 | **Image extraction** — Crops illustrations from pages, associates with nearest section |
| 7 | **LLM cleanup** — Claude CLI fixes OCR artifacts, restores French accents and ligatures |
| 8 | **Markdown assembly** — Writes structured markdown with frontmatter, references, and sections |
| 9 | **HTML generation** — Produces self-contained dark-themed gamebook player |

## CLI Options

| Option | Default | Description |
|--------|---------|-------------|
| `input` | required | Video file path |
| `--from-markdown PATH` | — | Skip video pipeline, generate HTML from existing markdown |
| `-l, --lang` | `fr` | Book language: `fr` or `en` |
| `--ref-pages N` | `0` | Number of initial pages as reference material (sidebar) |
| `-o, --output DIR` | `./<input-stem>/` | Output directory |
| `--ocr-engine` | `surya` | OCR engine: `surya` or `tesseract` |
| `--no-llm` | off | Skip Claude CLI cleanup pass |
| `--frame-interval` | `0.5` | Seconds between extracted frames |
| `--sharpness-threshold` | `50.0` | Laplacian variance below which = blurry |
| `--hash-threshold` | `8` | Max hamming distance for "same page" dedup |
| `--keep-frames` | off | Save selected page images to `output/frames/` |
| `--title TEXT` | auto | Book title for markdown frontmatter and HTML |
| `-v, --verbose` | off | Detailed progress to stderr |

## HTML Player Features

- **Left sidebar** (collapsible): reference page images (character sheet, equipment tables)
- **Top navigation bar** (sticky, scrollable): all § numbers, current section highlighted in gold
- **Main reading area**: serif font, dark theme, inline illustrations
- **Cross-references**: "rendez-vous au 147" / "go to 32" become clickable links
- **Self-contained**: single HTML file with all images base64-encoded, no external dependencies
- **All text selectable**: works with TTS tools (voice-speak, etc.)

## Markdown Format

The generated `sections.md` uses this format:

```markdown
---
title: Le Sorcier de la Montagne de Feu
lang: fr
ref_pages: 3
---

<!-- REF: Page 1 -->
![Reference Page 1](images/ref_001.jpg)

---

## § 1

Vous vous trouvez à l'entrée d'un sombre donjon...

Si vous voulez entrer, rendez-vous au 45.
Si vous préférez fuir, allez au 278.

---

## § 2

Le couloir mène à une salle immense...
```

## LLM Cleanup

Stage 7 calls the `claude` CLI to fix OCR artifacts. For French books, it specifically restores:
- Accents: é, è, ê, ë, à, â, ç, ù, û, î, ï, ô
- Ligatures: oeuvre → oeuvre, coeur → coeur, soeur → soeur

The LLM pass is optional (`--no-llm` to skip) and degrades gracefully if `claude` is not installed.

## Dependencies

- Python 3.10+
- OpenCV (frame extraction, image processing)
- Pillow + ImageHash (perceptual hashing)
- NumPy
- Surya OCR (default OCR engine)
- pytesseract (optional fallback OCR)
- `claude` CLI (optional, for LLM cleanup)
