#!/usr/bin/env python3
"""book_digitize.py — Extract text from a video of book page flipping.

Pipeline:
  1. Extract frames from video at configurable interval
  2. Detect pages, split two-page spreads, perspective correct, enhance
  3. Score frame quality, select sharpest per stable page view
  4. Deduplicate pages, extract page numbers, validate ordering
  5. OCR each unique page (Surya or Tesseract, French/English)
  6. Assemble ordered markdown or text file with page delimiters

Usage:
    book-digitize input.mp4 --output book.md --lang fra --frame-interval 0.5
"""

import argparse
import json
import os
import re
import shutil
import sys
import tempfile
import time
from pathlib import Path
from typing import NamedTuple

# ── Python version check ────────────────────────────────────────────────────
if sys.version_info < (3, 10):
    print(
        f"[book-digitize] ERROR: Python 3.10+ required (found {sys.version_info.major}.{sys.version_info.minor})",
        file=sys.stderr,
    )
    sys.exit(1)

import cv2
import imagehash
import numpy as np
from PIL import Image

# ── Constants ───────────────────────────────────────────────────────────────

DEFAULT_DIFF_THRESHOLD = 30.0
MIN_SEGMENT_FRAMES = 2
DEFAULT_SHARPNESS = 50.0
DEFAULT_HASH_THRESHOLD = 8
DEFAULT_FRAME_INTERVAL = 0.5
DEFAULT_PAGE_NUM_CROP_RATIO = 0.08
LOW_CONFIDENCE_THRESHOLD = 60.0
MIN_DISK_SPACE_MB = 500
MIN_PAGE_AREA_RATIO = 0.15  # minimum page area as fraction of frame

# Image detection constants
MIN_IMAGE_AREA_RATIO = 0.05       # minimum 5% of page area
MIN_IMAGE_ASPECT_RATIO = 0.3      # aspect ratio lower bound (w/h)
MAX_IMAGE_ASPECT_RATIO = 3.0      # aspect ratio upper bound (w/h)
IMAGE_PADDING_RATIO = 0.03        # 3% padding for cropping
CAPTION_PROXIMITY_RATIO = 0.03    # 3% of page height for caption search
FULL_PAGE_IMAGE_RATIO = 0.80      # single region covering >= 80% = full-page image
IMAGE_LABELS = {"Picture", "Figure", "Table"}  # Surya layout labels for images


# ── Data types ──────────────────────────────────────────────────────────────

class FrameInfo(NamedTuple):
    path: str
    timestamp: float
    index: int
    sharpness: float = 0.0


class ImageRegion(NamedTuple):
    bbox: tuple[int, int, int, int]   # (x1, y1, x2, y2) pixel coords
    page_index: int                    # index in ordered_pages
    figure_index: int                  # 0-based figure number on this page
    label: str                         # "Picture", "Figure", "Table", or "detected"
    caption: str                       # alt text (empty string if none)
    saved_path: str                    # path to cropped image file (empty until saved)


class PageResult(NamedTuple):
    text: str
    confidence: float
    page_number: int | None
    source_path: str
    images: list[ImageRegion] = []


class LayoutBlock(NamedTuple):
    block_type: str          # "heading1", "heading2", "paragraph", "caption", "image", "page_number"
    content: str             # text content, or image description for image blocks
    y_position: float        # 0.0-1.0 normalized vertical position on page
    bbox: tuple[int, int, int, int] | None  # pixel bbox for images, None for text


# ── Config ──────────────────────────────────────────────────────────────────

class Config:
    verbose: bool = False
    diff_threshold: float = DEFAULT_DIFF_THRESHOLD
    page_num_crop_ratio: float = DEFAULT_PAGE_NUM_CROP_RATIO
    log_file: str | None = None
    ocr_engine: str = "surya"
    no_preprocess: bool = False
    extract_images: bool = False
    claude_layout: bool = False
    max_claude_calls: int = 0
    pdf_margin: float = 2.0
    pdf_font: str = ""

    _log_fh = None

    @classmethod
    def open_log(cls) -> None:
        if cls.log_file:
            cls._log_fh = open(cls.log_file, "w", encoding="utf-8", buffering=1)

    @classmethod
    def close_log(cls) -> None:
        if cls._log_fh:
            cls._log_fh.close()
            cls._log_fh = None


def log(msg: str) -> None:
    line = f"[book-digitize] {msg}"
    print(line, file=sys.stderr)
    if Config._log_fh:
        Config._log_fh.write(line + "\n")


def vlog(msg: str) -> None:
    if Config.verbose:
        log(msg)


# ── Stage 1: Frame Extraction ──────────────────────────────────────────────

def extract_frames(video_path: str, output_dir: str, interval: float) -> list[FrameInfo]:
    """Extract frames from video at the given interval (seconds)."""
    cap = cv2.VideoCapture(video_path)
    if not cap.isOpened():
        log(f"ERROR: Cannot open video: {video_path}")
        sys.exit(1)

    fps = cap.get(cv2.CAP_PROP_FPS)
    total_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
    duration = total_frames / fps if fps > 0 else 0

    log(f"Stage 1/6: Extracting frames ({interval}s interval)...")  # Stage 1
    vlog(f"  Video: {fps:.1f} fps, {total_frames} frames, {duration:.1f}s duration")

    # Disk space check
    estimated_frames = int(duration / interval) if interval > 0 else total_frames
    try:
        stat = os.statvfs(output_dir)
        free_mb = (stat.f_bavail * stat.f_frsize) / (1024 * 1024)
        estimated_mb = estimated_frames * 0.5
        if free_mb < max(MIN_DISK_SPACE_MB, estimated_mb):
            log(f"  WARNING: Low disk space ({free_mb:.0f} MB free, ~{estimated_mb:.0f} MB needed)")
    except OSError:
        pass

    frame_step = max(1, int(fps * interval))
    frames = []
    frame_idx = 0

    while True:
        cap.set(cv2.CAP_PROP_POS_FRAMES, frame_idx)
        ret, frame = cap.read()
        if not ret:
            break

        timestamp = frame_idx / fps if fps > 0 else 0
        filename = f"frame_{frame_idx:06d}.jpg"
        filepath = os.path.join(output_dir, filename)
        cv2.imwrite(filepath, frame, [cv2.IMWRITE_JPEG_QUALITY, 95])

        frames.append(FrameInfo(path=filepath, timestamp=timestamp, index=frame_idx))
        frame_idx += frame_step

    cap.release()
    log(f"  -> {len(frames)} frames extracted ({duration:.0f}s video)")
    return frames


# ── Stage 1.5: Page Detection, Split & Enhancement ─────────────────────────

def _order_corners(pts: np.ndarray) -> np.ndarray:
    """Order 4 points as: top-left, top-right, bottom-right, bottom-left."""
    rect = np.zeros((4, 2), dtype=np.float32)
    s = pts.sum(axis=1)
    rect[0] = pts[np.argmin(s)]   # top-left has smallest sum
    rect[2] = pts[np.argmax(s)]   # bottom-right has largest sum
    d = np.diff(pts, axis=1)
    rect[1] = pts[np.argmin(d)]   # top-right has smallest difference
    rect[3] = pts[np.argmax(d)]   # bottom-left has largest difference
    return rect


def _perspective_transform(img: np.ndarray, corners: np.ndarray) -> np.ndarray:
    """Apply 4-point perspective transform to get a flat rectangular page."""
    rect = _order_corners(corners)
    tl, tr, br, bl = rect

    # Compute output dimensions
    width_top = np.linalg.norm(tr - tl)
    width_bot = np.linalg.norm(br - bl)
    max_w = int(max(width_top, width_bot))

    height_left = np.linalg.norm(bl - tl)
    height_right = np.linalg.norm(br - tr)
    max_h = int(max(height_left, height_right))

    if max_w < 50 or max_h < 50:
        return img

    dst = np.array([
        [0, 0], [max_w - 1, 0],
        [max_w - 1, max_h - 1], [0, max_h - 1],
    ], dtype=np.float32)

    M = cv2.getPerspectiveTransform(rect, dst)
    return cv2.warpPerspective(img, M, (max_w, max_h))


def _find_page_contour(img: np.ndarray) -> np.ndarray | None:
    """Find the largest rectangular contour that could be a book page."""
    gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
    blurred = cv2.GaussianBlur(gray, (5, 5), 0)
    edges = cv2.Canny(blurred, 30, 100)

    # Dilate to close gaps in edges
    kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (5, 5))
    edges = cv2.dilate(edges, kernel, iterations=2)

    contours, _ = cv2.findContours(edges, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)

    h, w = img.shape[:2]
    frame_area = h * w
    best = None
    best_area = 0

    for cnt in contours:
        area = cv2.contourArea(cnt)
        if area < frame_area * MIN_PAGE_AREA_RATIO:
            continue

        peri = cv2.arcLength(cnt, True)
        # Try progressively looser approximation to handle rounded corners
        approx = cv2.approxPolyDP(cnt, 0.02 * peri, True)
        if len(approx) != 4:
            approx = cv2.approxPolyDP(cnt, 0.04 * peri, True)
        if len(approx) != 4:
            approx = cv2.approxPolyDP(cnt, 0.06 * peri, True)

        if len(approx) == 4 and area > best_area:
            best = approx.reshape(4, 2).astype(np.float32)
            best_area = area

    return best


def _detect_gutter(gray: np.ndarray) -> int | None:
    """Find the vertical gutter (fold) in a two-page spread.

    Looks for the darkest vertical stripe in the central 40% of the image.
    """
    _, w = gray.shape
    center_start = int(w * 0.3)
    center_end = int(w * 0.7)
    center_region = gray[:, center_start:center_end]

    # Average brightness per column
    col_means = np.mean(center_region, axis=0)

    # Smooth to avoid noise
    kernel_size = max(5, w // 50)
    if kernel_size % 2 == 0:
        kernel_size += 1
    smoothed = cv2.GaussianBlur(col_means.reshape(1, -1), (kernel_size, 1), 0).flatten()

    # Find the darkest column (the gutter/fold)
    min_idx = np.argmin(smoothed)
    gutter_x = center_start + int(min_idx)

    # Verify: the gutter should be significantly darker than surroundings
    min_val = smoothed[min_idx]
    mean_val = np.mean(smoothed)
    if mean_val - min_val < 15:
        return None  # no clear gutter found

    # V-shape check: both sides of the minimum should be brighter
    region_len = len(smoothed)
    check_dist = max(1, region_len // 10)
    left_val = smoothed[max(0, min_idx - check_dist)]
    right_val = smoothed[min(region_len - 1, min_idx + check_dist)]
    if left_val - min_val < 8 or right_val - min_val < 8:
        return None  # not a clear V-shape dip — likely dark content, not a gutter

    return gutter_x


def _enhance_for_ocr(img: np.ndarray) -> np.ndarray:
    """Apply CLAHE + bilateral filter for better OCR.

    Preserves color channels when Surya is the OCR engine (it may benefit
    from color info). Converts to grayscale for Tesseract.
    """
    if Config.ocr_engine == "surya" and len(img.shape) == 3:
        # Preserve color: apply CLAHE on L channel of LAB space
        lab = cv2.cvtColor(img, cv2.COLOR_BGR2LAB)
        l_channel, a_channel, b_channel = cv2.split(lab)
        clahe = cv2.createCLAHE(clipLimit=2.0, tileGridSize=(8, 8))
        l_channel = clahe.apply(l_channel)
        enhanced = cv2.merge([l_channel, a_channel, b_channel])
        enhanced = cv2.cvtColor(enhanced, cv2.COLOR_LAB2BGR)
        enhanced = cv2.bilateralFilter(enhanced, 9, 75, 75)
        return enhanced

    # Grayscale path (Tesseract or already grayscale)
    if len(img.shape) == 3:
        gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
    else:
        gray = img

    clahe = cv2.createCLAHE(clipLimit=2.0, tileGridSize=(8, 8))
    enhanced = clahe.apply(gray)
    enhanced = cv2.bilateralFilter(enhanced, 9, 75, 75)
    return enhanced


def preprocess_frame(frame_path: str, output_dir: str, frame_idx: int) -> list[str]:
    """Detect page(s), split spreads, correct perspective, enhance.

    Returns list of paths to processed single-page images (1 or 2).
    """
    img = cv2.imread(frame_path)
    if img is None:
        return [frame_path]

    # Try to find a page contour
    page_contour = _find_page_contour(img)

    if page_contour is not None:
        # Found a rectangular page region — perspective correct it
        page_img = _perspective_transform(img, page_contour)
        vlog(f"  Frame {frame_idx}: page contour found, perspective corrected")
    else:
        page_img = img
        vlog(f"  Frame {frame_idx}: no page contour, using full frame")

    ph, pw = page_img.shape[:2]

    # Check if this looks like a two-page spread (wider than tall, or very wide)
    is_spread = pw > ph * 1.3

    if is_spread:
        gray = cv2.cvtColor(page_img, cv2.COLOR_BGR2GRAY) if len(page_img.shape) == 3 else page_img
        gutter = _detect_gutter(gray)
        if gutter is not None and gutter > pw * 0.2 and gutter < pw * 0.8:
            # Split into left and right pages
            margin = max(5, pw // 100)  # small margin to skip gutter shadow
            left_page = page_img[:, :max(1, gutter - margin)]
            right_page = page_img[:, min(pw - 1, gutter + margin):]

            results = []
            for side, page in [("L", left_page), ("R", right_page)]:
                enhanced = _enhance_for_ocr(page)
                path = os.path.join(output_dir, f"page_{frame_idx:06d}_{side}.jpg")
                cv2.imwrite(path, enhanced, [cv2.IMWRITE_JPEG_QUALITY, 95])
                results.append(path)

            vlog(f"  Frame {frame_idx}: split at gutter x={gutter}")
            return results

    # Single page — just enhance
    enhanced = _enhance_for_ocr(page_img)
    path = os.path.join(output_dir, f"page_{frame_idx:06d}.jpg")
    cv2.imwrite(path, enhanced, [cv2.IMWRITE_JPEG_QUALITY, 95])
    return [path]


def preprocess_frames(
    frames: list[FrameInfo],
    output_dir: str,
) -> list[FrameInfo]:
    """Run page detection + split + enhancement on all frames.

    Returns new list of FrameInfo pointing to processed page images.
    May be longer than input (two-page spreads produce 2 entries).
    """
    log("Stage 2/6: Detecting pages, splitting spreads, enhancing...")

    processed: list[FrameInfo] = []
    splits = 0

    for i, frame in enumerate(frames):
        page_paths = preprocess_frame(frame.path, output_dir, frame.index)
        for j, path in enumerate(page_paths):
            processed.append(FrameInfo(
                path=path,
                timestamp=frame.timestamp + j * 0.001,  # sub-ms offset for ordering
                index=frame.index * 10 + j,  # unique index
            ))
        if len(page_paths) > 1:
            splits += 1

        if (i + 1) % 20 == 0:
            log(f"  Preprocessing: {i + 1}/{len(frames)} frames...")

    log(f"  -> {len(processed)} page images ({splits} spreads split)")
    return processed


# ── Stage 2: Quality Scoring & Best-Frame Selection ────────────────────────

def compute_sharpness(image_path: str) -> float:
    """Laplacian variance — higher means sharper."""
    img = cv2.imread(image_path, cv2.IMREAD_GRAYSCALE)
    if img is None:
        return 0.0
    return cv2.Laplacian(img, cv2.CV_64F).var()


def compute_frame_diff(path_a: str, path_b: str) -> float:
    """Mean absolute pixel difference between two frames (grayscale, resized)."""
    size = (256, 256)
    a = cv2.imread(path_a, cv2.IMREAD_GRAYSCALE)
    b = cv2.imread(path_b, cv2.IMREAD_GRAYSCALE)
    if a is None or b is None:
        return 255.0
    a = cv2.resize(a, size)
    b = cv2.resize(b, size)
    return float(np.mean(np.abs(a.astype(np.float32) - b.astype(np.float32))))


def select_best_frames(
    frames: list[FrameInfo],
    sharpness_threshold: float,
) -> list[FrameInfo]:
    """Group frames into stable segments, pick the sharpest from each."""
    if not frames:
        return []

    log("Stage 3/6: Scoring frame quality...")

    scored = []
    for i, f in enumerate(frames):
        s = compute_sharpness(f.path)
        scored.append(FrameInfo(path=f.path, timestamp=f.timestamp, index=f.index, sharpness=s))
        vlog(f"  Frame {f.index}: sharpness={s:.1f}")
        if (i + 1) % 20 == 0:
            log(f"  Scoring: {i + 1}/{len(frames)} frames...")

    is_transitional = [False] * len(scored)
    for i in range(len(scored)):
        if scored[i].sharpness < sharpness_threshold:
            is_transitional[i] = True
        if i > 0:
            diff = compute_frame_diff(scored[i - 1].path, scored[i].path)
            if diff > Config.diff_threshold:
                is_transitional[i] = True
                vlog(f"  Frame {scored[i].index}: diff={diff:.1f} (transition)")

    segments: list[list[FrameInfo]] = []
    current_segment: list[FrameInfo] = []

    for i, frame in enumerate(scored):
        if is_transitional[i]:
            if len(current_segment) >= MIN_SEGMENT_FRAMES:
                segments.append(current_segment)
            current_segment = []
        else:
            current_segment.append(frame)

    if len(current_segment) >= MIN_SEGMENT_FRAMES:
        segments.append(current_segment)

    best = []
    for seg in segments:
        winner = max(seg, key=lambda f: f.sharpness)
        best.append(winner)
        vlog(f"  Segment ({len(seg)} frames): best={winner.index}, sharpness={winner.sharpness:.1f}")

    discarded = len(frames) - sum(len(s) for s in segments)
    log(f"  -> {len(best)} stable page views ({discarded} transitional frames discarded)")
    return best


# ── Stage 3: Page Deduplication & Ordering ──────────────────────────────────

def compute_phash(image_path: str) -> imagehash.ImageHash:
    """Perceptual hash of an image."""
    return imagehash.phash(Image.open(image_path))


def deduplicate_pages(
    frames: list[FrameInfo],
    hash_threshold: int,
) -> list[FrameInfo]:
    """Remove duplicate frames via consecutive + global perceptual hashing."""
    if not frames:
        return []

    log("Stage 4/6: Deduplicating pages...")

    hashes = [compute_phash(f.path) for f in frames]

    if len(frames) > 20:
        log(f"  Computed hashes for {len(frames)} frames")

    # Pass 1: consecutive dedup
    unique: list[FrameInfo] = [frames[0]]
    unique_hashes = [hashes[0]]

    for i in range(1, len(frames)):
        dist = hashes[i] - unique_hashes[-1]
        if dist <= hash_threshold:
            if frames[i].sharpness > unique[-1].sharpness:
                unique[-1] = frames[i]
                unique_hashes[-1] = hashes[i]
            vlog(f"  Frame {frames[i].index}: consecutive dup (dist={dist})")
        else:
            unique.append(frames[i])
            unique_hashes.append(hashes[i])
            vlog(f"  Frame {frames[i].index}: new page (dist={dist})")

    log(f"  -> {len(unique)} pages after consecutive dedup")

    # Pass 2: global dedup
    globally_unique: list[FrameInfo] = []
    seen_hashes: list[imagehash.ImageHash] = []
    removed = 0

    for i, frame in enumerate(unique):
        is_dup = False
        for seen_h in seen_hashes:
            if unique_hashes[i] - seen_h <= hash_threshold:
                is_dup = True
                vlog(f"  Frame {frame.index}: global dup of earlier page")
                removed += 1
                break
        if not is_dup:
            globally_unique.append(frame)
            seen_hashes.append(unique_hashes[i])

    if removed > 0:
        log(f"  -> {len(globally_unique)} pages after global dedup ({removed} revisited pages removed)")
    else:
        globally_unique = unique

    log(f"  -> {len(globally_unique)} unique pages")
    return globally_unique


def extract_page_number(image_path: str, lang: str) -> int | None:
    """Try to extract a printed page number from the top/bottom of the page.

    Uses Tesseract for speed (page number extraction is a small crop).
    Gracefully returns None if Tesseract is not available.
    """
    try:
        import pytesseract
    except ImportError:
        return None

    img = Image.open(image_path)
    w, h = img.size
    crop_h = int(h * Config.page_num_crop_ratio)

    strips = [
        img.crop((0, h - crop_h, w, h)),  # bottom strip
        img.crop((0, 0, w, crop_h)),       # top strip
    ]

    tess_lang = _surya_lang_to_tesseract(lang)

    for strip in strips:
        try:
            text = pytesseract.image_to_string(strip, lang=tess_lang, config="--psm 6").strip()
        except Exception:
            continue
        matches = re.findall(r"\b(\d{1,4})\b", text)
        if matches:
            candidates = [int(m) for m in matches]
            return max(candidates)

    return None


def _surya_lang_to_tesseract(lang: str) -> str:
    """Convert Surya language code to Tesseract code."""
    mapping = {"fr": "fra", "en": "eng"}
    parts = lang.split("+")
    converted = [mapping.get(p, p) for p in parts]
    return "+".join(converted)


def order_and_validate(
    pages: list[FrameInfo],
    lang: str,
) -> tuple[list[FrameInfo], list[int | None], list[str]]:
    """Extract page numbers, validate ordering, detect gaps."""
    warnings: list[str] = []

    page_numbers: list[int | None] = []
    for i, p in enumerate(pages):
        num = extract_page_number(p.path, lang)
        page_numbers.append(num)
        vlog(f"  Page {i + 1}: detected number = {num}")
        if (i + 1) % 10 == 0:
            log(f"  Extracting page numbers: {i + 1}/{len(pages)}...")

    detected = [(i, n) for i, n in enumerate(page_numbers) if n is not None]
    detected_count = len(detected)
    total = len(pages)

    log(f"  Page numbers detected: {detected_count}/{total}")

    if detected_count < total * 0.5:
        warnings.append(
            f"Only {detected_count}/{total} page numbers detected; using video order"
        )
        return pages, page_numbers, warnings

    nums = [n for _, n in detected]
    ascending_pairs = sum(1 for a, b in zip(nums, nums[1:]) if b > a)
    descending_pairs = sum(1 for a, b in zip(nums, nums[1:]) if b < a)

    if descending_pairs > ascending_pairs:
        pages = list(reversed(pages))
        page_numbers = list(reversed(page_numbers))
        warnings.append("Page order was reversed (video showed back-to-front)")
        log("  Detected reverse order — reversing pages")

    if detected_count > total * 0.8:
        numbered: list[tuple[int, FrameInfo]] = []
        unnumbered: list[tuple[int, FrameInfo]] = []

        for i, (p, n) in enumerate(zip(pages, page_numbers)):
            if n is not None:
                numbered.append((n, p))
            else:
                unnumbered.append((i, p))

        numbered.sort(key=lambda x: x[0])

        result_pages: list[FrameInfo] = []
        result_numbers: list[int | None] = []

        if unnumbered:
            unnumbered_insertions: list[tuple[float, FrameInfo]] = []
            for orig_idx, frame in unnumbered:
                prev_num = None
                next_num = None
                for j in range(orig_idx - 1, -1, -1):
                    if page_numbers[j] is not None:
                        prev_num = page_numbers[j]
                        break
                for j in range(orig_idx + 1, len(page_numbers)):
                    if page_numbers[j] is not None:
                        next_num = page_numbers[j]
                        break

                if prev_num is not None and next_num is not None:
                    sort_key = (prev_num + next_num) / 2.0
                elif prev_num is not None:
                    sort_key = prev_num + 0.5
                elif next_num is not None:
                    sort_key = next_num - 0.5
                else:
                    sort_key = float(orig_idx)

                unnumbered_insertions.append((sort_key, frame))

            all_entries: list[tuple[float, FrameInfo, int | None]] = []
            for n, p in numbered:
                all_entries.append((float(n), p, n))
            for sort_key, frame in unnumbered_insertions:
                all_entries.append((sort_key, frame, None))

            all_entries.sort(key=lambda x: x[0])

            for _, p, n in all_entries:
                result_pages.append(p)
                result_numbers.append(n)
        else:
            for n, p in numbered:
                result_pages.append(p)
                result_numbers.append(n)

        pages = result_pages
        page_numbers = result_numbers

    detected_sorted = sorted(n for n in page_numbers if n is not None)
    if len(detected_sorted) >= 2:
        for a, b in zip(detected_sorted, detected_sorted[1:]):
            if b - a > 1:
                missing = list(range(a + 1, b))
                if len(missing) <= 10:
                    warnings.append(f"Missing pages: {', '.join(str(m) for m in missing)} (between {a} and {b})")
                else:
                    warnings.append(f"Missing pages {a + 1}-{b - 1} (between {a} and {b})")

    seen: dict[int, int] = {}
    for i, n in enumerate(page_numbers):
        if n is not None:
            if n in seen:
                warnings.append(f"Duplicate page number {n} at positions {seen[n] + 1} and {i + 1}")
            seen[n] = i

    return pages, page_numbers, warnings


# ── Stage 4.5: Image Detection & Extraction ────────────────────────────────


def detect_image_regions_surya(
    image_path: str,
    page_index: int,
) -> tuple[list[ImageRegion], object]:
    """Detect image regions using Surya LayoutPredictor.

    Returns (regions, layout_result) — layout_result is cached for OCR reuse.
    """
    _, _, layout_pred = _init_surya()

    img = Image.open(image_path).convert("RGB")
    w, h = img.size
    page_area = w * h

    layout_results = layout_pred([img])
    layout = layout_results[0]

    # Collect header/footer bboxes to exclude overlapping image candidates
    exclude_bboxes = []
    for lb in layout.bboxes:
        if lb.label in ("PageHeader", "PageFooter"):
            exclude_bboxes.append(_bbox_from_polygon(lb.polygon))

    regions: list[ImageRegion] = []
    fig_idx = 0

    for lb in layout.bboxes:
        if lb.label not in IMAGE_LABELS:
            continue

        x1, y1, x2, y2 = _bbox_from_polygon(lb.polygon)
        x1, y1, x2, y2 = int(x1), int(y1), int(x2), int(y2)
        rw = x2 - x1
        rh = y2 - y1

        if rw <= 0 or rh <= 0:
            continue

        region_area = rw * rh
        if region_area < page_area * MIN_IMAGE_AREA_RATIO:
            continue

        aspect = rw / rh
        if aspect < MIN_IMAGE_ASPECT_RATIO or aspect > MAX_IMAGE_ASPECT_RATIO:
            continue

        # Skip regions overlapping header/footer
        region_bbox = (x1, y1, x2, y2)
        skip = False
        for ex_bbox in exclude_bboxes:
            if _boxes_overlap(region_bbox, ex_bbox, threshold=0.3):
                skip = True
                break
        if skip:
            continue

        regions.append(ImageRegion(
            bbox=(x1, y1, x2, y2),
            page_index=page_index,
            figure_index=fig_idx,
            label=lb.label,
            caption="",
            saved_path="",
        ))
        fig_idx += 1

    # Sort by vertical position
    regions.sort(key=lambda r: r.bbox[1])
    # Reassign figure indices after sort
    regions = [r._replace(figure_index=i) for i, r in enumerate(regions)]

    return regions, layout


def detect_image_regions_opencv(
    image_path: str,
    page_index: int,
) -> list[ImageRegion]:
    """Detect image regions using Tesseract block analysis + OpenCV verification.

    For use with Tesseract OCR engine where Surya layout is not available.
    Strategy: use Tesseract to identify text blocks, then find large vertical gaps
    between text blocks that contain non-trivial visual content (not whitespace).
    """
    import pytesseract

    img_gray = cv2.imread(image_path, cv2.IMREAD_GRAYSCALE)
    if img_gray is None:
        return []

    h, w = img_gray.shape
    page_area = w * h
    header_footer_margin = int(h * DEFAULT_PAGE_NUM_CROP_RATIO)
    img_pil = Image.open(image_path)

    # Get text block bounding boxes from Tesseract (PSM 3 = auto segmentation)
    try:
        tess_lang = _surya_lang_to_tesseract("en")
        data = pytesseract.image_to_data(
            img_pil, lang=tess_lang, config="--psm 3", output_type=pytesseract.Output.DICT
        )
    except Exception:
        return []

    # Collect text block bounding boxes (by block_num)
    block_boxes: dict[int, list[int]] = {}  # block_num -> [x1, y1, x2, y2]
    for i in range(len(data["text"])):
        conf = int(data["conf"][i])
        if conf < 0:
            continue
        word = data["text"][i].strip()
        if not word:
            continue
        bn = data["block_num"][i]
        bx = int(data["left"][i])
        by = int(data["top"][i])
        bw = int(data["width"][i])
        bh = int(data["height"][i])
        if bn not in block_boxes:
            block_boxes[bn] = [bx, by, bx + bw, by + bh]
        else:
            block_boxes[bn][0] = min(block_boxes[bn][0], bx)
            block_boxes[bn][1] = min(block_boxes[bn][1], by)
            block_boxes[bn][2] = max(block_boxes[bn][2], bx + bw)
            block_boxes[bn][3] = max(block_boxes[bn][3], by + bh)

    if not block_boxes:
        # No text detected at all — could be a full-page image
        if float(np.std(img_gray)) > 20.0:
            return [ImageRegion(
                bbox=(0, 0, w, h),
                page_index=page_index,
                figure_index=0,
                label="detected",
                caption="",
                saved_path="",
            )]
        return []

    # Sort text blocks by vertical position
    sorted_blocks = sorted(block_boxes.values(), key=lambda b: b[1])

    # Build a text coverage mask (vertical projection)
    text_y_ranges: list[tuple[int, int]] = []
    for box in sorted_blocks:
        text_y_ranges.append((box[1], box[3]))

    # Merge overlapping/adjacent text y-ranges
    merged: list[tuple[int, int]] = []
    for y1, y2 in sorted(text_y_ranges):
        if merged and y1 <= merged[-1][1] + 10:  # 10px tolerance
            merged[-1] = (merged[-1][0], max(merged[-1][1], y2))
        else:
            merged.append((y1, y2))

    # Find vertical gaps between text ranges
    gaps: list[tuple[int, int]] = []
    # Gap before first text block
    if merged and merged[0][0] > header_footer_margin:
        gaps.append((header_footer_margin, merged[0][0]))
    # Gaps between text blocks
    for i in range(1, len(merged)):
        gap_top = merged[i - 1][1]
        gap_bottom = merged[i][0]
        if gap_bottom > gap_top + 10:
            gaps.append((gap_top, gap_bottom))
    # Gap after last text block
    if merged and merged[-1][1] < h - header_footer_margin:
        gaps.append((merged[-1][1], h - header_footer_margin))

    # Check each gap for non-trivial visual content
    regions: list[ImageRegion] = []
    fig_idx = 0

    for gap_top, gap_bottom in gaps:
        gap_h = gap_bottom - gap_top
        # Use full page width for the candidate region
        x1, y1, x2, y2 = 0, gap_top, w, gap_bottom

        region_area = (x2 - x1) * (y2 - y1)
        if region_area < page_area * MIN_IMAGE_AREA_RATIO:
            continue

        # Skip thin horizontal strips (decorative rules)
        if gap_h < h * 0.03:
            continue

        # Verify the gap contains actual visual content (not just whitespace)
        roi = img_gray[y1:y2, x1:x2]
        roi_std = float(np.std(roi))
        if roi_std < 20.0:
            continue

        # Refine bounds: find the actual content bounding box within the gap
        _, roi_thresh = cv2.threshold(roi, 240, 255, cv2.THRESH_BINARY_INV)
        coords = cv2.findNonZero(roi_thresh)
        if coords is not None and len(coords) > 0:
            rx, ry, rrw, rrh = cv2.boundingRect(coords)
            content_area = rrw * rrh
            # Refine if content is at least 10% of the gap (catches centered figures)
            if content_area >= region_area * 0.10:
                x1 = rx
                y1 = gap_top + ry
                x2 = rx + rrw
                y2 = gap_top + ry + rrh

        rw = x2 - x1
        rh = y2 - y1
        region_area = rw * rh

        # Check area and aspect ratio after refinement
        if region_area < page_area * MIN_IMAGE_AREA_RATIO:
            continue

        if rh > 0:
            aspect = rw / rh
            if aspect < MIN_IMAGE_ASPECT_RATIO or aspect > MAX_IMAGE_ASPECT_RATIO:
                continue

        regions.append(ImageRegion(
            bbox=(x1, y1, x2, y2),
            page_index=page_index,
            figure_index=fig_idx,
            label="detected",
            caption="",
            saved_path="",
        ))
        fig_idx += 1

    # Sort by vertical position
    regions.sort(key=lambda r: r.bbox[1])
    regions = [r._replace(figure_index=i) for i, r in enumerate(regions)]

    return regions


def detect_image_regions(
    image_path: str,
    page_index: int,
) -> tuple[list[ImageRegion], object | None]:
    """Detect image regions using the configured engine's approach.

    Returns (regions, layout_result_or_None).
    """
    if Config.ocr_engine == "surya":
        return detect_image_regions_surya(image_path, page_index)
    else:
        return detect_image_regions_opencv(image_path, page_index), None


def crop_and_save_images(
    regions: list[ImageRegion],
    image_path: str,
    images_dir: str,
    page_label: str,
) -> list[ImageRegion]:
    """Crop detected image regions with padding and save to images/ dir.

    Returns new list with saved_path populated.
    """
    img = Image.open(image_path)
    w, h = img.size

    updated: list[ImageRegion] = []
    for region in regions:
        x1, y1, x2, y2 = region.bbox
        rw = x2 - x1
        rh = y2 - y1

        # Add padding (3% of region dimensions), clamped to image bounds
        pad_x = int(rw * IMAGE_PADDING_RATIO)
        pad_y = int(rh * IMAGE_PADDING_RATIO)
        px1 = max(0, x1 - pad_x)
        py1 = max(0, y1 - pad_y)
        px2 = min(w, x2 + pad_x)
        py2 = min(h, y2 + pad_y)

        cropped = img.crop((px1, py1, px2, py2))
        filename = f"page{page_label}_fig{region.figure_index + 1}.jpg"
        save_path = os.path.join(images_dir, filename)
        cropped.save(save_path, "JPEG", quality=95)

        updated.append(region._replace(saved_path=save_path))

    return updated


def annotate_page_with_boxes(
    image_path: str,
    regions: list[ImageRegion],
    output_path: str,
) -> None:
    """Draw bounding boxes on the full page image for debugging."""
    img = cv2.imread(image_path)
    if img is None:
        return

    for region in regions:
        x1, y1, x2, y2 = region.bbox
        cv2.rectangle(img, (x1, y1), (x2, y2), (0, 255, 0), 3)
        label = f"{region.label} #{region.figure_index + 1}"
        cv2.putText(img, label, (x1, max(y1 - 10, 0)),
                    cv2.FONT_HERSHEY_SIMPLEX, 0.7, (0, 255, 0), 2)

    cv2.imwrite(output_path, img)


def is_full_page_image(
    regions: list[ImageRegion],
    page_width: int,
    page_height: int,
) -> bool:
    """Check if a single image region covers >= 80% of the page area."""
    if len(regions) != 1:
        return False
    x1, y1, x2, y2 = regions[0].bbox
    region_area = (x2 - x1) * (y2 - y1)
    page_area = page_width * page_height
    return page_area > 0 and region_area >= page_area * FULL_PAGE_IMAGE_RATIO


def detect_captions(
    regions: list[ImageRegion],
    image_path: str,
    layout_boxes: list | None,
    lang: str,
) -> list[ImageRegion]:
    """Detect captions near image boundaries and attach them to regions.

    Surya path: uses Caption-labeled layout boxes within proximity of image edges.
    Tesseract path: OCR thin strips above/below each image region.
    """
    img = Image.open(image_path)
    page_w, page_h = img.size
    proximity = int(page_h * CAPTION_PROXIMITY_RATIO)

    updated: list[ImageRegion] = []

    for region in regions:
        _, ry1, _, ry2 = region.bbox
        caption = ""

        if layout_boxes is not None:
            # Surya path: find Caption layout boxes near the image
            for lb in layout_boxes:
                if lb.label != "Caption":
                    continue
                cb = _bbox_from_polygon(lb.polygon)
                cap_cy = (cb[1] + cb[3]) / 2  # caption vertical center

                # Caption is within proximity of image top or bottom edge
                if abs(cap_cy - ry2) <= proximity or abs(cap_cy - ry1) <= proximity:
                    cx1, cy1, cx2, cy2 = int(cb[0]), int(cb[1]), int(cb[2]), int(cb[3])
                    cap_crop = img.crop((max(0, cx1), max(0, cy1), cx2, cy2))
                    # Try Surya recognition first, fall back to Tesseract
                    try:
                        rec_pred, det_pred, _ = _init_surya()
                        ocr_results = rec_pred(
                            [cap_crop.convert("RGB")],
                            det_predictor=det_pred,
                            sort_lines=True,
                            math_mode=False,
                            return_words=False,
                        )
                        cap_text = " ".join(
                            line.text.strip() for line in ocr_results[0].text_lines
                            if line.text.strip()
                        )
                        if cap_text and len(cap_text) < 200:
                            caption = cap_text
                    except Exception:
                        # Fallback to Tesseract for caption OCR
                        try:
                            import pytesseract
                            tess_lang = _surya_lang_to_tesseract(lang)
                            cap_text = pytesseract.image_to_string(
                                cap_crop, lang=tess_lang, config="--psm 6"
                            ).strip()
                            if cap_text and len(cap_text) < 200:
                                caption = cap_text
                        except Exception:
                            vlog(f"  Caption extraction failed for region #{region.figure_index + 1}")
                    break
        else:
            # Tesseract path: OCR thin strips above/below image (full page width)
            try:
                import pytesseract
                tess_lang = _surya_lang_to_tesseract(lang)

                for strip_y1, strip_y2 in [
                    (max(0, ry1 - proximity), ry1),      # strip above
                    (ry2, min(page_h, ry2 + proximity)),  # strip below
                ]:
                    if strip_y2 <= strip_y1:
                        continue
                    # Use full page width to catch captions wider than the figure
                    strip = img.crop((0, strip_y1, page_w, strip_y2))
                    cap_text = pytesseract.image_to_string(
                        strip, lang=tess_lang, config="--psm 6"
                    ).strip()

                    if (cap_text and len(cap_text) < 120
                            and re.match(
                                r"^(Fig(ure|\.)?|Table|Illustration|Photo|Diagram|Schéma|Tableau)\b",
                                cap_text, re.IGNORECASE)):
                        caption = cap_text
                        break
            except Exception:
                pass

        updated.append(region._replace(caption=caption))

    return updated


# ── Stage 5: OCR ───────────────────────────────────────────────────────────

# -- Surya OCR --

_surya_rec_predictor = None
_surya_det_predictor = None
_surya_layout_predictor = None


def _init_surya():
    """Lazy-initialize Surya predictors (cached for reuse across pages)."""
    global _surya_rec_predictor, _surya_det_predictor, _surya_layout_predictor

    if _surya_rec_predictor is None:
        log("  Loading Surya models (first run downloads ~1-2 GB)...")
        try:
            from surya.foundation import FoundationPredictor
            from surya.recognition import RecognitionPredictor
            from surya.detection import DetectionPredictor
            from surya.layout import LayoutPredictor

            foundation = FoundationPredictor(device="cpu")
            # DetectionPredictor has its own model, not shared with foundation
            _surya_det_predictor = DetectionPredictor(device="cpu")
            _surya_rec_predictor = RecognitionPredictor(foundation)
            _surya_layout_predictor = LayoutPredictor(foundation)
            log("  Surya models loaded.")
        except Exception as e:
            log(f"ERROR: Failed to load Surya OCR models: {e}")
            log("  Check network connectivity and disk space (~2 GB needed in ~/.cache/)")
            log("  You can retry, or use --ocr-engine tesseract as fallback")
            sys.exit(1)

    return _surya_rec_predictor, _surya_det_predictor, _surya_layout_predictor


def ocr_page_surya(
    image_path: str,
    lang: str,
    output_format: str,
    image_regions: list[ImageRegion] | None = None,
    precomputed_layout: object | None = None,
) -> tuple[str, float, list[float]]:
    """OCR a page using Surya with layout detection.

    Returns (text_or_markdown, confidence, line_y_positions).
    line_y_positions: y-center of each output text line for positional assembly.
    """
    rec_pred, det_pred, layout_pred = _init_surya()

    img = Image.open(image_path).convert("RGB")

    # Reuse precomputed layout or run fresh
    if precomputed_layout is not None:
        layout = precomputed_layout
    else:
        layout_results = layout_pred([img])
        layout = layout_results[0]

    # Get OCR text with detection
    ocr_results = rec_pred(
        [img],
        det_predictor=det_pred,
        sort_lines=True,
        math_mode=False,
        return_words=False,
    )
    ocr_result = ocr_results[0]

    layout_boxes = layout.bboxes  # type: ignore[union-attr]

    # Filter out text lines overlapping image regions
    text_lines = ocr_result.text_lines
    if image_regions:
        filtered = []
        for line in text_lines:
            line_bbox = _bbox_from_polygon(line.polygon)
            overlaps = any(
                _boxes_overlap(line_bbox, r.bbox, threshold=0.3)
                for r in image_regions
            )
            if not overlaps:
                filtered.append(line)
        text_lines = filtered

    # Compute average confidence from filtered lines
    confidences = [line.confidence for line in text_lines if line.confidence is not None]
    mean_conf = float(np.mean(confidences)) * 100 if confidences else 0.0

    # Extract y-center positions for each text line (for positional assembly)
    line_y_positions = []
    for line in text_lines:
        bbox = _bbox_from_polygon(line.polygon)
        line_y_positions.append((bbox[1] + bbox[3]) / 2)

    if output_format == "md":
        return _build_surya_markdown(text_lines, layout_boxes), mean_conf, line_y_positions
    else:
        return _build_surya_plain(text_lines), mean_conf, line_y_positions


def _bbox_from_polygon(polygon: list[list[float]]) -> tuple[float, float, float, float]:
    """Convert polygon to (x1, y1, x2, y2) bounding box."""
    xs = [p[0] for p in polygon]
    ys = [p[1] for p in polygon]
    return min(xs), min(ys), max(xs), max(ys)


def _boxes_overlap(box_a: tuple, box_b: tuple, threshold: float = 0.3) -> bool:
    """Check if two bounding boxes overlap significantly."""
    ax1, ay1, ax2, ay2 = box_a
    bx1, by1, bx2, by2 = box_b

    ix1 = max(ax1, bx1)
    iy1 = max(ay1, by1)
    ix2 = min(ax2, bx2)
    iy2 = min(ay2, by2)

    if ix1 >= ix2 or iy1 >= iy2:
        return False

    intersection = (ix2 - ix1) * (iy2 - iy1)
    area_a = (ax2 - ax1) * (ay2 - ay1)

    if area_a <= 0:
        return False

    return intersection / area_a >= threshold


def _get_line_label(line_polygon, layout_boxes) -> str:
    """Find the layout label for an OCR text line based on spatial overlap."""
    line_bbox = _bbox_from_polygon(line_polygon)

    for lb in layout_boxes:
        lb_bbox = _bbox_from_polygon(lb.polygon)
        if _boxes_overlap(line_bbox, lb_bbox):
            return lb.label

    return "Text"


def _detect_paragraph_gaps(text_lines) -> set[int]:
    """Detect paragraph breaks by analyzing vertical gaps between OCR lines.

    Returns a set of line indices where a blank line should be inserted BEFORE.
    A gap is considered a paragraph break when it's >= 1.5x the median line spacing
    and the surrounding lines have reasonable confidence.
    """
    if len(text_lines) < 3:
        return set()

    # Compute vertical gaps between consecutive lines
    gaps: list[tuple[int, float]] = []  # (line_index, gap_pixels)
    for i in range(1, len(text_lines)):
        prev_bbox = _bbox_from_polygon(text_lines[i - 1].polygon)
        curr_bbox = _bbox_from_polygon(text_lines[i].polygon)
        # Gap = top of current line - bottom of previous line
        gap = curr_bbox[1] - prev_bbox[3]
        if gap > 0:
            gaps.append((i, gap))

    if not gaps:
        return set()

    # Median gap is the normal line spacing
    gap_values = [g for _, g in gaps]
    median_gap = float(np.median(gap_values))

    if median_gap <= 0:
        return set()

    # A paragraph break is a gap >= 1.5x the median
    PARA_GAP_RATIO = 1.5
    MIN_CONFIDENCE = 0.5

    para_breaks: set[int] = set()
    for line_idx, gap in gaps:
        if gap >= median_gap * PARA_GAP_RATIO:
            # Check confidence of surrounding lines
            prev_conf = text_lines[line_idx - 1].confidence or 0
            curr_conf = text_lines[line_idx].confidence or 0
            if prev_conf >= MIN_CONFIDENCE and curr_conf >= MIN_CONFIDENCE:
                para_breaks.add(line_idx)

    return para_breaks


def _build_surya_markdown(text_lines, layout_boxes) -> str:
    """Build markdown text from Surya OCR text lines + layout results."""
    lines_out: list[str] = []
    prev_label = None

    # Detect paragraph breaks from vertical spacing
    para_breaks = _detect_paragraph_gaps(text_lines)

    # Filter to only lines with text for index tracking
    text_line_indices: list[int] = []
    for i, line in enumerate(text_lines):
        if line.text.strip():
            text_line_indices.append(i)

    for pos, orig_idx in enumerate(text_line_indices):
        line = text_lines[orig_idx]
        text = line.text.strip()
        label = _get_line_label(line.polygon, layout_boxes)

        # Add paragraph break: layout region change OR vertical gap detected
        if pos > 0:
            if label != prev_label:
                lines_out.append("")
            elif orig_idx in para_breaks:
                lines_out.append("")

        if label == "SectionHeader":
            lines_out.append(f"## {text}")
        elif label == "Caption":
            lines_out.append(f"*{text}*")
        else:
            lines_out.append(text)

        prev_label = label

    return "\n".join(lines_out).strip()


def _build_surya_plain(text_lines) -> str:
    """Build plain text from Surya OCR text lines, preserving paragraph breaks."""
    # Detect paragraph breaks from vertical spacing
    para_breaks = _detect_paragraph_gaps(text_lines)

    lines_out: list[str] = []
    text_line_indices: list[int] = []
    for i, line in enumerate(text_lines):
        if line.text.strip():
            text_line_indices.append(i)

    for pos, orig_idx in enumerate(text_line_indices):
        line = text_lines[orig_idx]
        text = line.text.strip()

        if pos > 0 and orig_idx in para_breaks:
            lines_out.append("")

        lines_out.append(text)

    return "\n".join(lines_out).strip()


# -- Tesseract OCR (fallback) --

def ocr_page_tesseract(
    image_path: str,
    lang: str,
    output_format: str,
    image_regions: list[ImageRegion] | None = None,
) -> tuple[str, float, list[float]]:
    """OCR a page using Tesseract (fallback engine).

    Returns (text, confidence, line_y_positions).
    """
    import pytesseract

    img = Image.open(image_path)

    # White-fill image regions to exclude them from OCR
    if image_regions:
        img_cv = cv2.imread(image_path)
        for region in image_regions:
            x1, y1, x2, y2 = region.bbox
            cv2.rectangle(img_cv, (x1, y1), (x2, y2), (255, 255, 255), -1)
        img = Image.fromarray(cv2.cvtColor(img_cv, cv2.COLOR_BGR2RGB))

    tess_lang = _surya_lang_to_tesseract(lang)

    # Use PSM 3 (auto) when image regions were masked, since the page is no longer uniform
    psm_mode = "--psm 3" if image_regions else "--psm 6"
    data = pytesseract.image_to_data(
        img, lang=tess_lang, config=psm_mode, output_type=pytesseract.Output.DICT
    )

    # Group words by (block, par, line), track y-positions
    line_groups: dict[tuple[int, int, int], list[dict]] = {}
    line_tops: dict[tuple[int, int, int], list[int]] = {}
    all_confidences: list[int] = []

    for i in range(len(data["text"])):
        conf = int(data["conf"][i])
        if conf < 0:
            continue
        word = data["text"][i].strip()
        if not word:
            continue

        all_confidences.append(conf)
        key = (data["block_num"][i], data["par_num"][i], data["line_num"][i])
        if key not in line_groups:
            line_groups[key] = []
            line_tops[key] = []
        line_groups[key].append({
            "text": word,
            "height": int(data["height"][i]),
            "conf": conf,
        })
        line_tops[key].append(int(data["top"][i]) + int(data["height"][i]) // 2)

    output_lines: list[str] = []
    line_y_positions: list[float] = []
    prev_block = -1
    prev_par = -1

    if output_format == "md":
        # Compute body height for heading detection
        all_heights = []
        for words in line_groups.values():
            all_heights.extend(w["height"] for w in words)
        body_h = float(np.median(all_heights)) if all_heights else 0

        for (block, par, _line), words in sorted(line_groups.items()):
            if prev_block >= 0 and (block != prev_block or par != prev_par):
                output_lines.append("")
                line_y_positions.append(-1.0)  # blank line placeholder

            text = " ".join(w["text"] for w in words)
            median_h = float(np.median([w["height"] for w in words]))

            prefix = ""
            if body_h > 0 and median_h >= body_h * 1.8:
                prefix = "# "
            elif body_h > 0 and median_h >= body_h * 1.3:
                prefix = "## "

            output_lines.append(f"{prefix}{text}")
            tops = line_tops[(block, par, _line)]
            line_y_positions.append(float(np.mean(tops)) if tops else 0.0)
            prev_block = block
            prev_par = par
    else:
        for (block, par, _line), words in sorted(line_groups.items()):
            if prev_block >= 0 and (block != prev_block or par != prev_par):
                output_lines.append("")
                line_y_positions.append(-1.0)
            output_lines.append(" ".join(w["text"] for w in words))
            tops = line_tops[(block, par, _line)]
            line_y_positions.append(float(np.mean(tops)) if tops else 0.0)
            prev_block = block
            prev_par = par

    text = "\n".join(output_lines).strip()
    mean_conf = float(np.mean(all_confidences)) if all_confidences else 0.0
    return text, mean_conf, line_y_positions


# -- Image-text interleaving --

def interleave_images_with_text(
    text: str,
    image_regions: list[ImageRegion],
    line_y_positions: list[float],
    output_format: str,
) -> str:
    """Insert image references at correct vertical positions in the text.

    Uses y-coordinates of text lines and image regions to determine insertion points.
    """
    if not image_regions:
        return text

    lines = text.split("\n")

    # If y-positions don't match text lines, fall back to appending at end
    if not line_y_positions:
        vlog("  No line positions available, appending images at end of text")
        for region in image_regions:
            filename = os.path.basename(region.saved_path)
            alt_text = region.caption if region.caption else ""
            if output_format == "md":
                lines.append(f"\n![{alt_text}](images/{filename})\n")
            else:
                lines.append(f"\n[Image: images/{filename}]\n")
        return "\n".join(lines)

    # Build insertion map: for each image, find the line index to insert after
    insertions: list[tuple[int, ImageRegion]] = []  # (line_index, region)

    for region in sorted(image_regions, key=lambda r: r.bbox[1]):
        img_y_top = region.bbox[1]

        # Find the last text line whose y-center is above the image top
        insert_idx = 0
        for li, y_pos in enumerate(line_y_positions):
            if y_pos < 0:  # blank line placeholder
                continue
            if y_pos < img_y_top:
                insert_idx = li + 1
            else:
                break

        # Clamp to valid range
        insert_idx = min(insert_idx, len(lines))
        insertions.append((insert_idx, region))

    # Build image reference strings
    refs: list[tuple[int, str]] = []
    for insert_idx, region in insertions:
        filename = os.path.basename(region.saved_path)
        alt_text = region.caption if region.caption else ""

        if output_format == "md":
            ref = f"\n![{alt_text}](images/{filename})\n"
        else:
            ref = f"\n[Image: images/{filename}]\n"

        refs.append((insert_idx, ref))

    # Insert in reverse order to preserve line indices
    refs.sort(key=lambda r: r[0], reverse=True)
    for insert_idx, ref in refs:
        lines.insert(insert_idx, ref)

    return "\n".join(lines)


# -- OCR dispatcher --

def ocr_pages(
    pages: list[FrameInfo],
    lang: str,
    output_format: str = "md",
    page_image_regions: dict[int, list[ImageRegion]] | None = None,
    page_layouts: dict[int, object] | None = None,
    claude_layouts: dict[int, list[LayoutBlock]] | None = None,
) -> list[PageResult]:
    """OCR all pages using the configured engine."""
    engine = Config.ocr_engine
    total_stages = 7 if Config.extract_images else 6
    if Config.claude_layout:
        total_stages += 1
    stage_num = total_stages - 1 if Config.claude_layout else (6 if Config.extract_images else 5)
    log(f"Stage {stage_num}/{total_stages}: Running OCR (engine={engine}, lang={lang}, format={output_format})...")

    if engine == "tesseract":
        import pytesseract
        try:
            available = pytesseract.get_languages()
        except Exception:
            available = []

        tess_lang = _surya_lang_to_tesseract(lang)
        for l in tess_lang.split("+"):
            if available and l not in available:
                log(f"ERROR: Tesseract language '{l}' not installed.")
                log(f"  Available: {', '.join(available)}")
                log(f"  Install with: sudo apt-get install tesseract-ocr-{l}")
                sys.exit(1)

    results = []
    for i, page in enumerate(pages):
        regions = (page_image_regions or {}).get(i, [])
        layout = (page_layouts or {}).get(i)

        # Full-page image: skip OCR, output image reference only
        if regions:
            img = Image.open(page.path)
            pw, ph = img.size
            if is_full_page_image(regions, pw, ph):
                filename = os.path.basename(regions[0].saved_path)
                alt_text = regions[0].caption if regions[0].caption else ""
                if output_format == "md":
                    img_text = f"![{alt_text}](images/{filename})"
                else:
                    img_text = f"[Image: images/{filename}]"
                results.append(PageResult(
                    text=img_text,
                    confidence=100.0,
                    page_number=None,
                    source_path=page.path,
                    images=regions,
                ))
                log(f"  Page {i + 1}/{len(pages)}: full-page image, OCR skipped")
                continue

        # If Claude layout available for this page, use its text instead of OCR
        if claude_layouts and i in claude_layouts:
            text = _build_text_from_claude_layout(claude_layouts[i], output_format)
            confidence = 95.0  # Claude Vision confidence estimate
            line_y_positions = [
                b.y_position * 1000  # approximate pixel y for interleaving
                for b in claude_layouts[i] if b.block_type not in ("image", "page_number")
            ]
            if regions:
                text = interleave_images_with_text(text, regions, line_y_positions, output_format)
            results.append(PageResult(
                text=text, confidence=confidence, page_number=None,
                source_path=page.path, images=regions,
            ))
            log(f"  Page {i + 1}/{len(pages)}: {len(text)} chars from Claude Vision")
            continue

        # Run OCR with masking
        if engine == "surya":
            text, confidence, line_y_positions = ocr_page_surya(
                page.path, lang, output_format,
                image_regions=regions or None,
                precomputed_layout=layout,
            )
        else:
            text, confidence, line_y_positions = ocr_page_tesseract(
                page.path, lang, output_format,
                image_regions=regions or None,
            )

        # Interleave image references into text
        if regions:
            text = interleave_images_with_text(
                text, regions, line_y_positions, output_format,
            )

        results.append(PageResult(
            text=text,
            confidence=confidence,
            page_number=None,
            source_path=page.path,
            images=regions,
        ))
        status = "OK" if confidence >= LOW_CONFIDENCE_THRESHOLD else "LOW CONF"
        log(f"  Page {i + 1}/{len(pages)}: {len(text)} chars, confidence={confidence:.0f}% [{status}]")

    return results


# ── Claude Vision Layout Analysis ──────────────────────────────────────────


def _check_claude_cli() -> None:
    """Verify the claude CLI is available and authenticated."""
    import subprocess as sp
    try:
        result = sp.run(
            ["claude", "-p", "Reply with just OK"],
            capture_output=True, text=True, timeout=30,
        )
        if result.returncode != 0:
            stderr = result.stderr.strip()
            if "login" in stderr.lower() or "not logged in" in stderr.lower():
                log("ERROR: claude CLI is not logged in. Run 'claude' and log in first.")
            else:
                log(f"ERROR: claude CLI failed: {stderr}")
            sys.exit(1)
    except FileNotFoundError:
        log("ERROR: 'claude' CLI not found. Install Claude Code first.")
        sys.exit(1)


def _load_claude_cache(cache_path: str) -> dict:
    """Load cached Claude responses from JSON file."""
    if os.path.isfile(cache_path):
        with open(cache_path, "r", encoding="utf-8") as f:
            return json.load(f)
    return {}


def _save_claude_cache(cache_path: str, cache: dict) -> None:
    """Write cache dict to JSON file atomically."""
    tmp_path = cache_path + ".tmp"
    with open(tmp_path, "w", encoding="utf-8") as f:
        json.dump(cache, f, ensure_ascii=False)
    os.replace(tmp_path, cache_path)


def _parse_claude_blocks(raw_blocks: list[dict]) -> list[LayoutBlock]:
    """Parse raw JSON blocks into LayoutBlock list."""
    blocks = []
    for b in raw_blocks:
        block_type = b.get("type", "paragraph")
        content = b.get("content", "")
        y_pos = float(b.get("y_position", 0.0))
        bbox_raw = b.get("bbox")
        bbox = tuple(bbox_raw) if bbox_raw and len(bbox_raw) == 4 else None
        blocks.append(LayoutBlock(
            block_type=block_type,
            content=content,
            y_position=y_pos,
            bbox=bbox,
        ))
    return blocks


def _call_claude_cli(image_path: str) -> str | None:
    """Call claude CLI to analyze a page image. Returns raw text response or None."""
    import subprocess as sp

    prompt = (
        f"Read the image at {image_path} and analyze this book page. "
        "Extract all content blocks in reading order. "
        "For each block, provide its type, text content, and vertical position as a "
        "fraction (0.0 = top of page, 1.0 = bottom).\n\n"
        "Output JSON with this exact schema:\n"
        '{"blocks": [{"type": "heading1"|"heading2"|"paragraph"|"caption"|"page_number"|"image", '
        '"content": "the text content (empty string for image blocks)", '
        '"y_position": 0.15, '
        '"bbox": [x1, y1, x2, y2] or null}]}\n\n'
        "Rules:\n"
        '- For image blocks: set type to "image", content to a brief description, '
        "bbox to the pixel bounding box [x1, y1, x2, y2]\n"
        "- For text blocks: bbox should be null, y_position is the vertical center of the block\n"
        "- Preserve the original language of the text exactly as written\n"
        "- Merge hyphenated line breaks within paragraphs\n"
        "- Output ONLY the JSON object, no other text"
    )

    try:
        result = sp.run(
            ["claude", "-p", prompt,
             "--allowedTools", "Read",
             "--dangerously-skip-permissions",
             "--model", "sonnet"],
            capture_output=True, text=True, timeout=120,
        )
        if result.returncode != 0:
            return None
        return result.stdout.strip()
    except (sp.TimeoutExpired, FileNotFoundError, OSError):
        return None


def run_claude_layout_analysis(
    pages: list[FrameInfo],
    cache_path: str,
    max_calls: int = 0,
) -> dict[int, list[LayoutBlock]]:
    """Run Claude Vision on each page via the claude CLI."""
    _check_claude_cli()
    cache = _load_claude_cache(cache_path)
    results: dict[int, list[LayoutBlock]] = {}
    calls_made = 0

    # Restore cached results
    for key, val in cache.items():
        idx = int(key)
        results[idx] = _parse_claude_blocks(val.get("blocks", []))

    for i, page in enumerate(pages):
        if i in results:
            continue
        if max_calls > 0 and calls_made >= max_calls:
            log(f"  Page {i + 1}: skipped (budget cap of {max_calls} calls reached)")
            continue

        # Call Claude CLI with retries
        response_json = None
        for attempt in range(3):
            raw_text = _call_claude_cli(page.path)
            if raw_text is None:
                wait = min(2 ** attempt, 8)
                log(f"  Page {i + 1}: claude CLI failed, retrying in {wait}s...")
                time.sleep(wait)
                continue

            # Strip markdown code fences if present
            if raw_text.startswith("```"):
                raw_text = re.sub(r"^```\w*\n?", "", raw_text)
                raw_text = re.sub(r"\n?```$", "", raw_text)

            try:
                response_json = json.loads(raw_text)
                calls_made += 1
                break
            except json.JSONDecodeError as e:
                log(f"  Page {i + 1}: Claude returned invalid JSON: {e}")
                break

        if response_json and "blocks" in response_json:
            blocks = _parse_claude_blocks(response_json["blocks"])
            results[i] = blocks
            cache[str(i)] = response_json
            _save_claude_cache(cache_path, cache)
            log(f"  Page {i + 1}/{len(pages)}: {len(blocks)} blocks from Claude Vision")
        else:
            log(f"  Page {i + 1}/{len(pages)}: Claude failed, will fall back to OCR")

    return results


def _build_text_from_claude_layout(
    blocks: list[LayoutBlock],
    output_format: str,
) -> str:
    """Convert Claude LayoutBlocks into markdown or plain text."""
    lines: list[str] = []
    for block in blocks:
        if block.block_type == "page_number":
            continue
        if block.block_type == "image":
            continue  # images handled separately via interleaving
        if block.block_type == "heading1":
            if output_format == "md":
                lines.append(f"# {block.content}")
            else:
                lines.append(block.content)
        elif block.block_type == "heading2":
            if output_format == "md":
                lines.append(f"## {block.content}")
            else:
                lines.append(block.content)
        elif block.block_type == "caption":
            if output_format == "md":
                lines.append(f"*{block.content}*")
            else:
                lines.append(block.content)
        else:
            lines.append(block.content)
        lines.append("")
    return "\n".join(lines).strip()


# ── PDF Output ─────────────────────────────────────────────────────────────

def _setup_pdf_fonts(pdf, font_path: str) -> str:
    """Set up fonts for PDF. Returns the font family name to use."""
    if font_path and os.path.isfile(font_path):
        pdf.add_font("CustomFont", "", font_path)
        pdf.add_font("CustomFont", "B", font_path)
        pdf.add_font("CustomFont", "I", font_path)
        return "CustomFont"

    serif_candidates = [
        ("/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
         "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf",
         "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Italic.ttf"),
        ("/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf",
         "/usr/share/fonts/truetype/liberation/LiberationSerif-Bold.ttf",
         "/usr/share/fonts/truetype/liberation/LiberationSerif-Italic.ttf"),
        ("/usr/share/fonts/truetype/noto/NotoSerif-Regular.ttf",
         "/usr/share/fonts/truetype/noto/NotoSerif-Bold.ttf",
         "/usr/share/fonts/truetype/noto/NotoSerif-Italic.ttf"),
        ("/usr/share/fonts/truetype/freefont/FreeSerif.ttf",
         "/usr/share/fonts/truetype/freefont/FreeSerifBold.ttf",
         "/usr/share/fonts/truetype/freefont/FreeSerifItalic.ttf"),
    ]

    for regular, bold, italic in serif_candidates:
        if os.path.isfile(regular):
            family = Path(regular).stem.replace("-Regular", "").replace("-regular", "")
            pdf.add_font(family, "", regular)
            if os.path.isfile(bold):
                pdf.add_font(family, "B", bold)
            if os.path.isfile(italic):
                pdf.add_font(family, "I", italic)
            log(f"  PDF font: {regular}")
            return family

    log("  WARNING: No TrueType serif font found. Using built-in Times (limited Unicode).")
    log("  Install dejavu or liberation fonts for full Unicode support.")
    return "Times"


def _parse_text_to_blocks(
    text: str,
    images: list[ImageRegion],
) -> list[LayoutBlock]:
    """Parse OCR markdown/text into LayoutBlock list for the non-Claude path."""
    blocks: list[LayoutBlock] = []
    lines = text.split("\n")
    # Build filename->ImageRegion map for matching
    image_map: dict[str, ImageRegion] = {}
    for img in images:
        if img.saved_path:
            image_map[os.path.basename(img.saved_path)] = img

    current_paragraph: list[str] = []

    def flush_paragraph():
        if current_paragraph:
            para_text = "\n".join(current_paragraph).strip()
            if para_text:
                blocks.append(LayoutBlock("paragraph", para_text, 0.0, None))
            current_paragraph.clear()

    for line in lines:
        stripped = line.strip()

        # Markdown image reference
        img_match = re.match(r"^!\[([^\]]*)\]\(images/([^)]+)\)$", stripped)
        if img_match:
            flush_paragraph()
            filename = img_match.group(2)
            caption = img_match.group(1)
            region = image_map.get(filename)
            bbox = region.bbox if region else None
            blocks.append(LayoutBlock("image", caption, 0.0, bbox))
            continue

        # Plain text image reference
        img_match2 = re.match(r"^\[Image: images/([^\]]+)\]$", stripped)
        if img_match2:
            flush_paragraph()
            filename = img_match2.group(1)
            region = image_map.get(filename)
            bbox = region.bbox if region else None
            blocks.append(LayoutBlock("image", "", 0.0, bbox))
            continue

        # Headings
        if stripped.startswith("## "):
            flush_paragraph()
            blocks.append(LayoutBlock("heading2", stripped[3:], 0.0, None))
            continue
        if stripped.startswith("# "):
            flush_paragraph()
            blocks.append(LayoutBlock("heading1", stripped[2:], 0.0, None))
            continue

        # Italic caption
        if stripped.startswith("*") and stripped.endswith("*") and len(stripped) > 2 and not stripped.startswith("**"):
            flush_paragraph()
            blocks.append(LayoutBlock("caption", stripped[1:-1], 0.0, None))
            continue

        # Empty line = paragraph break
        if not stripped:
            flush_paragraph()
            continue

        current_paragraph.append(stripped)

    flush_paragraph()

    # Assign sequential y_positions for flow layout
    if blocks:
        for i, block in enumerate(blocks):
            blocks[i] = block._replace(y_position=(i + 0.5) / len(blocks))

    return blocks


def _estimate_text_lines(pdf, text: str, usable_w: float) -> int:
    """Estimate line count for text, handling embedded newlines."""
    import math
    total_lines = 0
    for line in text.split("\n"):
        if not line.strip():
            total_lines += 1
            continue
        text_w = pdf.get_string_width(line)
        total_lines += max(1, math.ceil(text_w / usable_w))
    return max(1, total_lines)


def _estimate_content_height(
    blocks: list[LayoutBlock],
    pdf,
    usable_w: float,
    font_family: str,
    body_size: float,
    h1_size: float,
    h2_size: float,
    caption_size: float,
    images: list[ImageRegion],
    usable_h: float,
) -> float:
    """Estimate total content height in mm for a list of blocks."""
    total = 0.0

    for block in blocks:
        if block.block_type == "heading1":
            pdf.set_font(font_family, "B", h1_size)
            line_h = h1_size * 0.5
            n_lines = _estimate_text_lines(pdf, block.content, usable_w)
            total += n_lines * line_h + h1_size * 0.3
        elif block.block_type == "heading2":
            pdf.set_font(font_family, "B", h2_size)
            line_h = h2_size * 0.5
            n_lines = _estimate_text_lines(pdf, block.content, usable_w)
            total += n_lines * line_h + h2_size * 0.3
        elif block.block_type == "caption":
            pdf.set_font(font_family, "I", caption_size)
            line_h = caption_size * 0.45
            n_lines = _estimate_text_lines(pdf, block.content, usable_w)
            total += n_lines * line_h + 1.0
        elif block.block_type == "image":
            matched = None
            for img in images:
                if img.saved_path and os.path.isfile(img.saved_path):
                    if not matched:
                        matched = img
                    if block.bbox and img.bbox == block.bbox:
                        matched = img
                        break
            if matched and matched.saved_path and os.path.isfile(matched.saved_path):
                pil_img = Image.open(matched.saved_path)
                iw, ih = pil_img.size
                render_w = usable_w
                render_h = render_w * (ih / iw) if iw > 0 else 50.0
                max_img_h = usable_h * 0.6
                if render_h > max_img_h:
                    render_h = max_img_h
                total += render_h + 3.0
            else:
                total += 30.0
        else:  # paragraph
            pdf.set_font(font_family, "", body_size)
            line_h = body_size * 0.45
            n_lines = _estimate_text_lines(pdf, block.content, usable_w)
            total += n_lines * line_h + body_size * 0.15

    return total * 1.05  # 5% safety margin


def _render_pdf_page(
    pdf,
    blocks: list[LayoutBlock],
    images: list[ImageRegion],
    page_number: int | str | None,
    usable_w: float,
    usable_h: float,
    margin_mm: float,
    font_family: str,
    body_size: float,
    h1_size: float,
    h2_size: float,
    caption_size: float,
) -> None:
    """Render content blocks onto the current PDF page."""
    # Build image lookup: match by bbox or sequentially
    image_files: list[str] = []
    for img in images:
        if img.saved_path and os.path.isfile(img.saved_path):
            image_files.append(img.saved_path)
    img_idx = 0
    used_images: set[str] = set()

    page_w = pdf.w
    max_y = pdf.h - margin_mm - 10.0  # page number area

    for block in blocks:
        x = margin_mm
        current_y = pdf.get_y()

        # Overflow protection: stop rendering if past page boundary
        if current_y >= max_y:
            break

        if block.block_type == "heading1":
            pdf.set_font(font_family, "B", h1_size)
            line_h = h1_size * 0.5
            pdf.set_y(current_y + h1_size * 0.2)
            pdf.set_x(x)
            pdf.multi_cell(usable_w, line_h, block.content)
            pdf.ln(h1_size * 0.1)

        elif block.block_type == "heading2":
            pdf.set_font(font_family, "B", h2_size)
            line_h = h2_size * 0.5
            pdf.set_y(current_y + h2_size * 0.15)
            pdf.set_x(x)
            pdf.multi_cell(usable_w, line_h, block.content)
            pdf.ln(h2_size * 0.1)

        elif block.block_type == "caption":
            pdf.set_font(font_family, "I", caption_size)
            line_h = caption_size * 0.45
            pdf.set_x(x)
            pdf.multi_cell(usable_w, line_h, block.content)
            pdf.ln(1.0)

        elif block.block_type == "image":
            # Find the matching image file
            img_path = None
            # Match by bbox if available
            if block.bbox:
                for img in images:
                    if img.saved_path and img.bbox == block.bbox and os.path.isfile(img.saved_path):
                        img_path = img.saved_path
                        used_images.add(img_path)
                        break
            # Fallback: next unused sequential image
            if not img_path:
                while img_idx < len(image_files):
                    candidate = image_files[img_idx]
                    img_idx += 1
                    if candidate not in used_images:
                        img_path = candidate
                        used_images.add(img_path)
                        break

            if img_path and os.path.isfile(img_path):
                pil_img = Image.open(img_path)
                iw, ih = pil_img.size
                render_w = usable_w
                render_h = render_w * (ih / iw) if iw > 0 else 50.0
                max_img_h = usable_h * 0.6
                if render_h > max_img_h:
                    render_h = max_img_h
                    render_w = render_h * (iw / ih) if ih > 0 else usable_w
                # Don't render if image would overflow
                if pdf.get_y() + render_h > max_y:
                    render_h = max(10.0, max_y - pdf.get_y())
                    render_w = render_h * (iw / ih) if ih > 0 else usable_w
                # Center image horizontally
                img_x = margin_mm + (usable_w - render_w) / 2
                pdf.image(img_path, x=img_x, y=pdf.get_y(), w=render_w, h=render_h)
                pdf.set_y(pdf.get_y() + render_h + 1.5)

        elif block.block_type == "page_number":
            pass  # handled separately at bottom

        else:  # paragraph
            pdf.set_font(font_family, "", body_size)
            line_h = body_size * 0.45
            pdf.set_x(x)
            pdf.multi_cell(usable_w, line_h, block.content)
            pdf.ln(body_size * 0.1)

    # Page number at bottom center
    if page_number is not None:
        pdf.set_y(pdf.h - margin_mm - 3.0)
        pdf.set_font(font_family, "", caption_size)
        pdf.set_x(0)
        pdf.cell(page_w, 3.0, str(page_number), align="C")


def write_output_pdf(
    results: list[PageResult],
    page_numbers: list[int | None],
    warnings: list[str],
    output_path: str,
    ordered_pages: list[FrameInfo],
    claude_layouts: dict[int, list[LayoutBlock]] | None = None,
    margin_cm: float = 2.0,
    font_path: str = "",
) -> None:
    """Write final output as a multi-page PDF."""
    from fpdf import FPDF

    total_stages = 7 if Config.extract_images else 6
    if Config.claude_layout:
        total_stages += 1
    log(f"Stage {total_stages}/{total_stages}: Writing PDF output...")

    pdf = FPDF()
    pdf.set_auto_page_break(auto=False)

    # Metadata
    pdf.set_title(Path(output_path).stem)
    pdf.set_author("book-digitize")
    # fpdf2 sets creation date automatically

    # Fonts
    font_family = _setup_pdf_fonts(pdf, font_path)

    margin_mm = margin_cm * 10.0
    body_size = 11.0
    h1_size = 18.0
    h2_size = 14.0
    caption_size = 9.0
    min_font_size = 6.0

    for i, result in enumerate(results):
        pnum = page_numbers[i] if i < len(page_numbers) else None
        label = pnum if pnum is not None else i + 1

        # Determine page dimensions from source image aspect ratio
        src_path = ordered_pages[i].path if i < len(ordered_pages) else None
        page_w_mm = 210.0  # A4 width default
        page_h_mm = 297.0  # A4 height default

        if src_path and os.path.isfile(src_path):
            src_img = Image.open(src_path)
            sw, sh = src_img.size
            if sw > 0 and sh > 0:
                aspect = sh / sw
                page_h_mm = page_w_mm * aspect
                page_h_mm = max(200.0, min(400.0, page_h_mm))

        pdf.add_page(format=(page_w_mm, page_h_mm))
        usable_w = page_w_mm - 2 * margin_mm
        usable_h = page_h_mm - 2 * margin_mm - 10.0  # 10mm for page number

        pdf.set_margins(margin_mm, margin_mm, margin_mm)
        pdf.set_xy(margin_mm, margin_mm)

        # Build layout blocks
        images = result.images or []
        if claude_layouts and i in claude_layouts:
            blocks = claude_layouts[i]
        else:
            blocks = _parse_text_to_blocks(result.text, images)

        if not blocks:
            # Empty page — just render page number
            _render_pdf_page(
                pdf, [], images, label,
                usable_w, usable_h, margin_mm, font_family,
                body_size, h1_size, h2_size, caption_size,
            )
            continue

        # Estimate content height and compute scale factor
        est_height = _estimate_content_height(
            blocks, pdf, usable_w, font_family,
            body_size, h1_size, h2_size, caption_size,
            images, usable_h,
        )

        scale = 1.0
        if est_height > usable_h:
            scale = usable_h / est_height
            if body_size * scale < min_font_size:
                scale = min_font_size / body_size

        scaled_body = body_size * scale
        scaled_h1 = max(h1_size * scale, min_font_size)
        scaled_h2 = max(h2_size * scale, min_font_size)
        scaled_cap = max(caption_size * scale, min_font_size)

        _render_pdf_page(
            pdf, blocks, images, label,
            usable_w, usable_h, margin_mm, font_family,
            scaled_body, scaled_h1, scaled_h2, scaled_cap,
        )

        vlog(f"  PDF page {i + 1}: scale={scale:.2f}, blocks={len(blocks)}")

    pdf.output(output_path)

    # Summary
    total = len(results)
    detected = sum(1 for n in page_numbers if n is not None)
    total_images = sum(len(r.images) for r in results)

    log("")
    log("=" * 50)
    log("  Summary")
    log("=" * 50)
    log(f"  Total pages: {total}")
    log(f"  Pages with detected numbers: {detected}/{total}")
    log(f"  OCR engine: {'Claude Vision' if Config.claude_layout else Config.ocr_engine}")
    if total_images > 0:
        log(f"  Images embedded: {total_images}")
    if warnings:
        for w in warnings:
            log(f"  WARNING: {w}")
    log(f"  PDF written to: {output_path}")


# ── Stage 6: Output Assembly ────────────────────────────────────────────────

def write_output(
    results: list[PageResult],
    page_numbers: list[int | None],
    warnings: list[str],
    output_path: str,
    output_format: str = "md",
) -> None:
    """Write final output file and summary log."""
    total_stages = 7 if Config.extract_images else 6
    log(f"Stage {total_stages}/{total_stages}: Writing output...")

    with open(output_path, "w", encoding="utf-8") as f:
        for i, result in enumerate(results):
            pnum = page_numbers[i] if i < len(page_numbers) else None
            label = str(pnum) if pnum is not None else f"({i + 1})"

            if output_format == "md":
                f.write(f"---\n\n**Page {label}**\n\n")
            else:
                f.write(f"--- Page {label} ---\n")

            f.write(result.text)
            f.write("\n\n")

    total = len(results)
    detected = sum(1 for n in page_numbers if n is not None)
    low_conf = [
        (i, r.confidence)
        for i, r in enumerate(results)
        if r.confidence < LOW_CONFIDENCE_THRESHOLD
    ]

    log("")
    log("=" * 50)
    log("  Summary")
    log("=" * 50)
    log(f"  Total pages extracted: {total}")
    log(f"  Pages with detected numbers: {detected}/{total}")
    log(f"  OCR engine: {Config.ocr_engine}")

    total_images = sum(len(r.images) for r in results)
    if total_images > 0:
        log(f"  Images extracted: {total_images}")

    if warnings:
        for w in warnings:
            log(f"  WARNING: {w}")

    if low_conf:
        parts = [f"page {i + 1} ({c:.0f}%)" for i, c in low_conf]
        log(f"  Low-confidence pages: {', '.join(parts)}")

    log(f"  Output written to: {output_path}")


# ── CLI ─────────────────────────────────────────────────────────────────────

def parse_interval(value: str) -> float:
    """Parse frame interval, stripping optional 's'/'sec'/'seconds' suffix."""
    value = value.strip().lower()
    value = re.sub(r"\s*(seconds|sec|s)$", "", value)
    try:
        result = float(value)
    except ValueError:
        raise argparse.ArgumentTypeError(f"Invalid frame interval: '{value}'")
    if result <= 0:
        raise argparse.ArgumentTypeError("Frame interval must be positive")
    return result


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="book-digitize",
        description="Extract text from a video of book page flipping.",
    )
    parser.add_argument("input", help="Path to video file (MP4, MKV, AVI)")
    parser.add_argument(
        "-o", "--output",
        help="Output file (default: <input_stem>.md or .txt)",
    )
    parser.add_argument(
        "-f", "--format",
        choices=["md", "txt", "pdf"],
        default="md",
        help="Output format: md (markdown), txt (plain text), or pdf (digital book) (default: md)",
    )
    parser.add_argument(
        "-l", "--lang",
        default="fr",
        help="Language code: fr, en, fr+en (Surya codes) or fra, eng (Tesseract codes) (default: fr)",
    )
    parser.add_argument(
        "--ocr-engine",
        choices=["surya", "tesseract"],
        default="surya",
        help="OCR engine (default: surya — much more accurate, slower on CPU)",
    )
    parser.add_argument(
        "--no-preprocess",
        action="store_true",
        help="Skip page detection/split/enhancement (use raw frames)",
    )
    parser.add_argument(
        "--frame-interval",
        type=parse_interval,
        default=DEFAULT_FRAME_INTERVAL,
        help=f"Seconds between extracted frames (default: {DEFAULT_FRAME_INTERVAL})",
    )
    parser.add_argument(
        "--sharpness-threshold",
        type=float,
        default=DEFAULT_SHARPNESS,
        help=f"Laplacian variance below which a frame is blurry (default: {DEFAULT_SHARPNESS})",
    )
    parser.add_argument(
        "--diff-threshold",
        type=float,
        default=DEFAULT_DIFF_THRESHOLD,
        help=f"Frame diff above which = page transition (default: {DEFAULT_DIFF_THRESHOLD})",
    )
    parser.add_argument(
        "--hash-threshold",
        type=int,
        default=DEFAULT_HASH_THRESHOLD,
        help=f"Max hamming distance to consider same page (default: {DEFAULT_HASH_THRESHOLD})",
    )
    parser.add_argument(
        "--page-crop-ratio",
        type=float,
        default=DEFAULT_PAGE_NUM_CROP_RATIO,
        help=f"Top/bottom fraction for page number search (default: {DEFAULT_PAGE_NUM_CROP_RATIO})",
    )
    parser.add_argument(
        "--extract-images",
        action="store_true",
        help="Detect and extract embedded images (photos, diagrams, figures) into images/ directory",
    )
    parser.add_argument(
        "--claude-layout",
        action="store_true",
        help="Use Claude Vision for page layout analysis (requires ANTHROPIC_API_KEY)",
    )
    parser.add_argument(
        "--pdf-margin",
        type=float,
        default=2.0,
        help="PDF margin in cm (default: 2.0)",
    )
    parser.add_argument(
        "--pdf-font",
        default="",
        help="Path to TTF font file for PDF body text (default: auto-detect system serif)",
    )
    parser.add_argument(
        "--max-claude-calls",
        type=int,
        default=0,
        help="Max Claude API calls for --claude-layout, 0=unlimited (default: 0)",
    )
    parser.add_argument(
        "--keep-frames",
        action="store_true",
        help="Save selected page frames to ./frames/ for debugging",
    )
    parser.add_argument(
        "--log",
        dest="log_file",
        help="Write summary log to this file (in addition to stderr)",
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Print detailed progress to stderr",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    Config.verbose = args.verbose
    Config.diff_threshold = args.diff_threshold
    Config.page_num_crop_ratio = args.page_crop_ratio
    Config.ocr_engine = args.ocr_engine
    Config.no_preprocess = args.no_preprocess
    Config.extract_images = args.extract_images
    Config.claude_layout = args.claude_layout
    Config.max_claude_calls = args.max_claude_calls
    Config.pdf_margin = args.pdf_margin
    Config.pdf_font = args.pdf_font

    # Auto-enable image extraction for PDF output
    if args.format == "pdf":
        Config.extract_images = True
        args.extract_images = True

    if args.log_file:
        Config.log_file = args.log_file
    Config.open_log()

    try:
        _run_pipeline(args)
    finally:
        Config.close_log()


def _run_pipeline(args: argparse.Namespace) -> None:
    ext_map = {"md": ".md", "txt": ".txt", "pdf": ".pdf"}
    ext = ext_map.get(args.format, ".md")
    if args.output is None:
        args.output = Path(args.input).stem + ext

    if not os.path.isfile(args.input):
        log(f"ERROR: Input file not found: {args.input}")
        sys.exit(1)

    with tempfile.TemporaryDirectory(prefix="book-digitize-") as tmpdir:
        # Stage 1: Extract frames
        frames = extract_frames(args.input, tmpdir, args.frame_interval)
        if not frames:
            log("ERROR: No frames extracted from video.")
            sys.exit(1)

        # Stage 1.5: Preprocess (page detection, split, enhance)
        if not Config.no_preprocess:
            processed_dir = os.path.join(tmpdir, "processed")
            os.makedirs(processed_dir)
            frames = preprocess_frames(frames, processed_dir)

        # Stage 2: Score quality, select best frames
        best_frames = select_best_frames(frames, args.sharpness_threshold)
        if not best_frames:
            log("WARNING: No stable page views found. Falling back to sharpest frames.")
            scored = [
                FrameInfo(f.path, f.timestamp, f.index, compute_sharpness(f.path))
                for f in frames
            ]
            scored.sort(key=lambda f: f.sharpness, reverse=True)
            best_frames = scored[:max(1, len(scored) // 4)]
            best_frames.sort(key=lambda f: f.timestamp)

        # Stage 3: Deduplicate and order
        unique_pages = deduplicate_pages(best_frames, args.hash_threshold)
        ordered_pages, page_numbers, warnings = order_and_validate(unique_pages, args.lang)

        # Optionally save frames for debugging
        if args.keep_frames:
            frames_dir = Path("frames")
            frames_dir.mkdir(exist_ok=True)
            for i, page in enumerate(ordered_pages):
                pnum = page_numbers[i] if i < len(page_numbers) and page_numbers[i] is not None else None
                # Use index prefix to avoid filename collisions on duplicate page numbers
                label = f"{i + 1:03d}_p{pnum}" if pnum is not None else f"{i + 1:03d}"
                dest = frames_dir / f"page_{label}.jpg"
                shutil.copy2(page.path, dest)
            log(f"  Saved {len(ordered_pages)} page frames to ./frames/")

        # Stage 4.5: Image detection & extraction (optional)
        page_image_regions: dict[int, list[ImageRegion]] = {}
        page_layouts: dict[int, object] = {}

        if args.extract_images:
            images_dir = str(Path(args.output).parent / "images")
            if Path(args.output).parent == Path():
                images_dir = "images"
            os.makedirs(images_dir, exist_ok=True)

            total_stages = 7
            log(f"Stage 5/{total_stages}: Detecting and extracting images...")

            for i, page in enumerate(ordered_pages):
                regions, layout_result = detect_image_regions(page.path, i)

                if layout_result is not None:
                    page_layouts[i] = layout_result

                if regions:
                    # Detect captions (layout_result is a LayoutResult with .bboxes)
                    layout_boxes = getattr(layout_result, "bboxes", None) if layout_result is not None else None
                    regions = detect_captions(regions, page.path, layout_boxes, args.lang)

                    # Crop and save images (use index prefix to avoid collisions on duplicate page numbers)
                    pnum = page_numbers[i] if i < len(page_numbers) and page_numbers[i] is not None else None
                    page_label = f"{i + 1}p{pnum}" if pnum is not None else str(i + 1)
                    regions = crop_and_save_images(regions, page.path, images_dir, page_label)

                    # Annotate for debugging (only if --keep-frames already created the dir)
                    if args.keep_frames:
                        ann_label = f"{i + 1:03d}_p{pnum}" if pnum is not None else f"{i + 1:03d}"
                        annotated_path = str(Path("frames") / f"page_{ann_label}_annotated.jpg")
                        annotate_page_with_boxes(page.path, regions, annotated_path)

                    page_image_regions[i] = regions
                    vlog(f"  Page {i + 1}: {len(regions)} image(s) detected")

            total_images = sum(len(r) for r in page_image_regions.values())
            log(f"  Found {total_images} image(s) across {len(page_image_regions)} page(s)")

        # Stage: Claude Vision layout analysis (optional)
        claude_layouts: dict[int, list[LayoutBlock]] = {}
        if Config.claude_layout:
            total_stages = 7 if Config.extract_images else 6
            total_stages += 1
            cache_path = args.output + ".claude_cache.json"
            log(f"Stage {total_stages - 1}/{total_stages}: Running Claude Vision layout analysis...")
            claude_layouts = run_claude_layout_analysis(
                ordered_pages, cache_path, Config.max_claude_calls,
            )

        # Stage: OCR
        # For PDF with Claude layout, use "md" format for OCR text (we parse it ourselves)
        ocr_format = args.format if args.format != "pdf" else "md"
        results = ocr_pages(
            ordered_pages, args.lang, ocr_format,
            page_image_regions=page_image_regions or None,
            page_layouts=page_layouts or None,
            claude_layouts=claude_layouts or None,
        )

        # Stage: Assemble output
        if args.format == "pdf":
            write_output_pdf(
                results, page_numbers, warnings, args.output,
                ordered_pages,
                claude_layouts=claude_layouts or None,
                margin_cm=Config.pdf_margin,
                font_path=Config.pdf_font,
            )
        else:
            write_output(results, page_numbers, warnings, args.output, args.format)


if __name__ == "__main__":
    main()
