# book-digitize

CLI tool that takes a video recording of someone flipping through a book and produces a clean, ordered **markdown**, **plain text**, or **PDF** file — ready for reading or text-to-speech consumption.

## How It Works

```
input.mp4
    |
    v
+----------------------+
| Stage 1: Extract     |  OpenCV VideoCapture -> JPEG frames every N seconds
|         Frames       |
+----------+-----------+
           v
+----------------------+
| Stage 1.5: Preprocess|  Detect pages, split two-page spreads,
|                      |  perspective correct, enhance
+----------+-----------+
           v
+----------------------+
| Stage 2: Score &     |  Laplacian variance (sharpness) + frame-to-frame diff
|    Select Best       |  -> group into stable segments, pick sharpest per group
|        Frames        |  -> discard transitional/blurry frames (page flips)
+----------+-----------+
           v
+----------------------+
| Stage 3: Deduplicate |  Perceptual hash (phash) -- consecutive + global dedup
|    & Order Pages     |  -> one frame per unique page
|                      |  -> OCR header/footer for page numbers
|                      |  -> validate ordering, flag missing pages
+----------+-----------+
           v
+----------------------+
| Stage 4.5: Images    |  (optional) Detect & extract embedded images
|  (--extract-images)  |  -> Surya layout detection or OpenCV contour fallback
|                      |  -> crop, save, detect captions
+----------+-----------+
           v
+----------------------------+
| Stage 4.75: Claude Vision  |  (optional, --claude-layout)
|                            |  -> send each page to Claude Vision API
|                            |  -> structured layout: headings, paragraphs, images
|                            |  -> replaces OCR text with Claude's extraction
+----------+-----------------+
           v
+----------------------+
| Stage 5: OCR         |  Surya (default, more accurate) or Tesseract
|                      |  -> markdown: detects headings
|                      |  -> preserves paragraph structure
|                      |  -> interleaves image references
+----------+-----------+
           v
+----------------------+
| Stage 6: Assemble    |  -> book.md / book.txt / book.pdf
|         Output       |  -> summary log (page count, gaps, low-confidence)
+----------------------+
```

## Installation

```bash
cd /path/to/book-digitize
chmod +x install.sh
./install.sh
```

The installer will:
1. Check for `python3` and `tesseract`
2. Install Tesseract language packs (`eng`, `fra`) if missing
3. Create an isolated Python venv at `~/.local/share/book-digitize/venv/`
4. Install Python dependencies (OpenCV, pytesseract, Pillow, ImageHash, numpy, surya-ocr, fpdf2, anthropic)
5. Install the `book-digitize` command to `~/.local/bin/`

**Note:** Surya OCR downloads ~1-2 GB of model weights on first run (cached in `~/.cache/huggingface/`).

### System Requirements

- **Linux** (Debian/Ubuntu/Mint or Arch-based)
- **Python 3.10+**
- **Tesseract 5.x** (`sudo apt install tesseract-ocr`)
- For best PDF output: a TrueType serif font (DejaVu Serif, Liberation Serif, or Noto Serif). Most distros include one by default.

## Usage

```bash
# Basic -- French text, markdown output (Surya OCR)
book-digitize input.mp4

# English book, custom output path
book-digitize input.mp4 --output book.md --lang en

# Plain text output (no heading detection)
book-digitize input.mp4 --format txt --output book.txt

# PDF output (auto-enables image extraction)
book-digitize input.mp4 --format pdf --output book.pdf

# PDF with Claude Vision layout analysis (highest quality, uses claude CLI)
book-digitize input.mp4 --format pdf --claude-layout

# PDF with Claude Vision, budget-capped at 10 calls
book-digitize input.mp4 --format pdf --claude-layout --max-claude-calls 10

# Extract embedded images (photos, diagrams)
book-digitize input.mp4 --extract-images

# Use Tesseract instead of Surya
book-digitize input.mp4 --ocr-engine tesseract --lang fra

# Both languages (auto-detect per page)
book-digitize input.mp4 --lang fr+en

# Debug: save selected page images
book-digitize input.mp4 --keep-frames --verbose
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `input` (positional) | required | Path to video file (MP4, MKV, AVI) |
| `-o, --output` | `<input>.md` | Output file path |
| `-f, --format` | `md` | Output format: `md` (markdown), `txt` (plain text), or `pdf` (digital book) |
| `-l, --lang` | `fr` | Language: `fr`, `en`, `fr+en` (Surya) or `fra`, `eng` (Tesseract) |
| `--ocr-engine` | `surya` | OCR engine: `surya` (more accurate, slower on CPU) or `tesseract` |
| `--no-preprocess` | off | Skip page detection/split/enhancement (use raw frames) |
| `--extract-images` | off | Detect and extract embedded images into `images/` directory. Auto-enabled for `--format pdf`. |
| `--claude-layout` | off | Use Claude Vision for page layout analysis (requires `claude` CLI logged in) |
| `--pdf-margin` | `2.0` | PDF margin in cm |
| `--pdf-font` | auto-detect | Path to a TTF font file for PDF body text |
| `--max-claude-calls` | `0` (unlimited) | Budget cap on Claude API calls |
| `--frame-interval` | `0.5` | Seconds between extracted frames (accepts `0.5s`, `0.5sec`, or `0.5`) |
| `--sharpness-threshold` | `50.0` | Laplacian variance below which = blurry |
| `--diff-threshold` | `30.0` | Frame pixel diff above which = page transition |
| `--hash-threshold` | `8` | Max hamming distance to consider "same page" |
| `--page-crop-ratio` | `0.08` | Fraction of page height to search for page numbers |
| `--keep-frames` | off | Save selected page images to `./frames/` |
| `--log` | off | Write summary log to a file (in addition to stderr) |
| `-v, --verbose` | off | Detailed progress to stderr |

## Output Formats

### Markdown (`--format md`)

Detects headings by analyzing word bounding box heights. Lines significantly taller than body text become `#` or `##` headings. Embedded images appear as `![caption](images/filename.jpg)`.

```markdown
---

**Page 1**

# AVANT-PROPOS

La question d'une suite se pose depuis plus de 5 ans au moment
ou j'ecris ces lignes.

![Figure 1: Diagramme](images/page_1p1_fig1.jpg)

---

**Page 2**

Le succes du premier livre a ete une surprise...
```

### Plain Text (`--format txt`)

```
--- Page 1 ---
AVANT-PROPOS

La question d'une suite se pose depuis plus de 5 ans...

--- Page 2 ---
Le succes du premier livre...
```

### PDF (`--format pdf`)

Produces a multi-page PDF where each book page becomes one PDF page:
- **Text**: Serif font (DejaVu Serif by default), 11pt body, 18pt H1, 14pt H2
- **Images**: Placed at detected vertical position, scaled to page width
- **Captions**: Italic 9pt below each image
- **Page numbers**: Bottom-center of each page
- **Page size**: Matches the aspect ratio of the captured page frame
- **Overflow handling**: Font auto-scales down (minimum 6pt) if content exceeds page height

Image extraction is auto-enabled for PDF output.

### Claude Vision Layout (`--claude-layout`)

When enabled, each page frame is sent to Claude Sonnet via the Anthropic API for structured layout analysis. Claude identifies headings, paragraphs, images, and captions with precise vertical positions — producing higher-quality PDF output than OCR alone.

Requirements:
- The `claude` CLI must be installed and logged in (uses your Claude Max subscription)
- Responses are cached alongside the output file (`.claude_cache.json`) so re-runs don't repeat calls
- Retries with exponential backoff on failures
- Use `--max-claude-calls` to cap usage

## Summary Output

A summary is printed to stderr after every run:

```
[book-digitize] Summary
[book-digitize]   Total pages extracted: 32
[book-digitize]   Pages with detected numbers: 28/32
[book-digitize]   OCR engine: surya
[book-digitize]   Images extracted: 5
[book-digitize]   WARNING: Missing pages: 6, 7 (between 5 and 8)
[book-digitize]   Low-confidence pages: page 3 (45%), page 15 (52%)
[book-digitize]   Output written to: book.md
```

## Tuning Tips

- **Blurry frames still selected?** Increase `--sharpness-threshold` (try 80-100).
- **Too few pages detected?** Decrease `--sharpness-threshold` (try 20-30) or decrease `--frame-interval` to 0.3.
- **Duplicate pages in output?** Decrease `--hash-threshold` (try 4-6).
- **Pages merged together?** Increase `--hash-threshold` (try 10-12).
- **Fast page flipping?** Decrease `--frame-interval` to 0.2-0.3s.
- **Page numbers not detected?** Increase `--page-crop-ratio` (try 0.12-0.15) if numbers are far from the edge.
- **False page transitions?** Increase `--diff-threshold` (try 40-50) if lighting flickers cause false splits.
- **Want a persistent log?** Use `--log run.log` to save the summary alongside stderr output.
- **Don't need headings?** Use `--format txt` for plain text output.
- **OCR too slow on CPU?** Use `--ocr-engine tesseract` for faster (but less accurate) results.
- **PDF text too small?** Reduce content per page or increase `--pdf-margin` slightly.
- **Missing accents in PDF?** Install a Unicode serif font: `sudo apt install fonts-dejavu`.

## Dependencies

| Library | Purpose |
|---------|---------|
| `opencv-python-headless` | Video frame extraction, sharpness scoring, frame differencing |
| `pytesseract` | Python wrapper for Tesseract OCR |
| `Pillow` | Image loading and cropping |
| `ImageHash` | Perceptual hashing for page deduplication |
| `numpy` | Numerical operations (used by OpenCV and ImageHash) |
| `surya-ocr` | Advanced OCR engine with layout detection (default, more accurate than Tesseract) |
| `fpdf2` | PDF generation (pure Python, no system dependencies) |
| `claude` CLI | Claude Code CLI for vision layout analysis (not a pip package — install separately) |

## Known Limitations

- **Two-page spreads**: Automatic page splitting works but may not be perfect on all layouts.
- **Curved pages**: Text near the spine may be distorted by page curvature. No dewarping is applied.
- **Multi-column text**: Tesseract `--psm 6` assumes a single column. Surya handles columns better.
- **Bold/italic detection**: OCR engines do not reliably detect font styles. Only font *size* (headings) is detected.
- **Handwritten annotations**: OCR is optimized for printed text only.
- **Roman numeral page numbers**: Not detected (digits only).
- **Large videos**: A 2-hour video at 0.5s intervals produces ~14,400 frames (~7 GB temp disk space). A disk space warning is shown if free space is low.
- **Claude Vision usage**: Each page is one `claude` CLI invocation. A 300-page book = 300 calls. Use `--max-claude-calls` to limit.
