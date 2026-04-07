#!/usr/bin/env python3
"""Extract text from an image using Tesseract OCR.

Usage:
    python3 ocr_extract.py <image_path>

Prints extracted text to stdout.  All diagnostics go to stderr.
"""
import sys

from PIL import Image
import pytesseract


def main():
    if len(sys.argv) < 2:
        print("Usage: ocr_extract.py <image_path>", file=sys.stderr)
        sys.exit(1)

    image_path = sys.argv[1]

    print(f"[ocr] Loading image: {image_path}", file=sys.stderr)
    img = Image.open(image_path)

    print("[ocr] Running Tesseract…", file=sys.stderr)
    text = pytesseract.image_to_string(img).strip()

    print(f"[ocr] Extracted {len(text)} characters", file=sys.stderr)
    print(text)


if __name__ == "__main__":
    main()
