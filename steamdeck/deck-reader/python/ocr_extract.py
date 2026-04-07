#!/usr/bin/env python3
"""Extract text from an image using Tesseract OCR.

Usage:
    python3 ocr_extract.py <image_path> [lang] [cleanup|raw]

    image_path  Path to the image file to process.
    lang        Tesseract language code(s), e.g. "eng", "eng+jpn". Default: "eng".
    cleanup     "cleanup" (default) to clean OCR artifacts, "raw" to skip.

Prints extracted text to stdout.  All diagnostics go to stderr.
"""
import re
import sys

from PIL import Image
import pytesseract


# Characters that are clearly artifacts in dialogue text.
# Keeps: . , ! ? ' " : ; - … — ( ) and alphanumerics.
_STRAY_SYMBOLS = re.compile(r'[\\|/~^{}\[\]<>]')

# Common Tesseract h↔b misreads in English.
_OCR_FIXES = {
    'tbe': 'the', 'witb': 'with', 'bave': 'have',
    'tben': 'then', 'wben': 'when', 'tbat': 'that',
    'tbis': 'this', 'wbich': 'which', 'tbey': 'they',
}
_OCR_FIX_RE = {
    re.compile(r'\b' + wrong + r'\b', re.IGNORECASE): right
    for wrong, right in _OCR_FIXES.items()
}


def clean_ocr_text(text, lang="eng"):
    """Remove common Tesseract OCR artifacts from dialogue text."""
    # 1. Strip stray symbols that don't belong in dialogue
    text = _STRAY_SYMBOLS.sub('', text)

    # 2. Normalize repeated punctuation
    text = re.sub(r'\.{3,}', '…', text)
    text = re.sub(r'…\.+', '…', text)
    text = re.sub(r'-{2,}', '—', text)

    # 3. Collapse multiple spaces
    text = re.sub(r' {2,}', ' ', text)

    # 4. Fix common English OCR misreads (only for English text)
    if lang.startswith("eng"):
        for pattern, replacement in _OCR_FIX_RE.items():
            text = pattern.sub(replacement, text)

    # 5. Remove lines that are only punctuation/whitespace (no letters)
    lines = text.split('\n')
    cleaned = []
    for line in lines:
        stripped = line.strip()
        if not stripped:
            cleaned.append('')
            continue
        alpha_count = sum(1 for c in stripped if c.isalpha())
        if alpha_count >= 2 or len(stripped) <= 1:
            cleaned.append(line.rstrip())
        # else: drop the line (noise — symbols/digits only)

    # 6. Collapse 3+ consecutive blank lines to 2
    text = '\n'.join(cleaned).strip()
    text = re.sub(r'\n{3,}', '\n\n', text)

    return text


def main():
    if len(sys.argv) < 2:
        print("Usage: ocr_extract.py <image_path> [lang] [cleanup|raw]", file=sys.stderr)
        sys.exit(1)

    image_path = sys.argv[1]
    lang = sys.argv[2] if len(sys.argv) >= 3 else "eng"
    cleanup = (sys.argv[3] if len(sys.argv) >= 4 else "cleanup") != "raw"

    print(f"[ocr] Loading image: {image_path}", file=sys.stderr)
    img = Image.open(image_path)

    print(f"[ocr] Running Tesseract (lang={lang})…", file=sys.stderr)
    text = pytesseract.image_to_string(img, lang=lang).strip()

    if cleanup and text:
        text = clean_ocr_text(text, lang)

    print(f"[ocr] Extracted {len(text)} characters", file=sys.stderr)
    print(text)


if __name__ == "__main__":
    main()
