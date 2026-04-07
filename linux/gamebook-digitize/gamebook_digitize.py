#!/usr/bin/env python3
"""gamebook-digitize -- Convert gamebook video to markdown + interactive HTML."""

import sys

if sys.version_info < (3, 10):
    print(f"ERROR: Python 3.10+ required (found {sys.version})", file=sys.stderr)
    sys.exit(1)

import argparse
import os
import re
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import NamedTuple

import cv2
import imagehash
import numpy as np
from PIL import Image


# ── Data Structures ─────────────────────────────────────────────────────────


class FrameInfo(NamedTuple):
    path: str
    timestamp: float
    index: int
    sharpness: float = 0.0


class OcrLine(NamedTuple):
    text: str
    bbox: tuple[float, float, float, float]  # (x1, y1, x2, y2)
    height: float  # y2-y1, font size proxy
    confidence: float
    page_idx: int
    label: str  # "SectionHeader", "Text", etc.


class ImageRegion(NamedTuple):
    bbox: tuple[int, int, int, int]
    page_idx: int
    saved_path: str


class Section(NamedTuple):
    number: int
    lines: list[str]
    images: list[str]  # paths relative to images/
    page_range: tuple[int, int]


# ── Config & Logging ────────────────────────────────────────────────────────


class Config:
    verbose = False
    ocr_engine = "surya"
    no_llm = False
    ref_pages = 0


def log(msg: str) -> None:
    print(f"[gamebook-digitize] {msg}", file=sys.stderr)


def vlog(msg: str) -> None:
    if Config.verbose:
        log(msg)


# ── Stage 1: Frame Extraction ──────────────────────────────────────────────


def extract_frames(
    video_path: str, output_dir: str, interval: float
) -> list[FrameInfo]:
    """Extract frames from video at the given interval (seconds)."""
    cap = cv2.VideoCapture(video_path)
    if not cap.isOpened():
        log(f"ERROR: Cannot open video file: {video_path}")
        sys.exit(1)

    fps = cap.get(cv2.CAP_PROP_FPS)
    total_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
    frame_step = max(1, int(fps * interval))

    vlog(f"Video: {fps:.1f} fps, {total_frames} total frames, step={frame_step}")

    frames: list[FrameInfo] = []
    i = 0
    for frame_idx in range(0, total_frames, frame_step):
        cap.set(cv2.CAP_PROP_POS_FRAMES, frame_idx)
        ret, frame = cap.read()
        if not ret:
            vlog(f"  Could not read frame at index {frame_idx}, stopping")
            break

        path = os.path.join(output_dir, f"frame_{i:06d}.jpg")
        cv2.imwrite(path, frame, [cv2.IMWRITE_JPEG_QUALITY, 95])
        timestamp = frame_idx / fps
        frames.append(FrameInfo(path=path, timestamp=timestamp, index=i, sharpness=0.0))
        i += 1

    cap.release()
    log(f"  Extracted {len(frames)} frames from video")
    return frames


# ── Stage 2: Best Frame Selection ──────────────────────────────────────────


def compute_sharpness(image_path: str) -> float:
    """Compute sharpness score via Laplacian variance."""
    img = cv2.imread(image_path)
    if img is None:
        return 0.0
    gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
    return cv2.Laplacian(gray, cv2.CV_64F).var()


def compute_frame_diff(path_a: str, path_b: str) -> float:
    """Compute mean absolute pixel difference between two frames (grayscale, resized)."""
    img_a = cv2.imread(path_a, cv2.IMREAD_GRAYSCALE)
    img_b = cv2.imread(path_b, cv2.IMREAD_GRAYSCALE)
    if img_a is None or img_b is None:
        return 999.0
    a = cv2.resize(img_a, (256, 256)).astype(float)
    b = cv2.resize(img_b, (256, 256)).astype(float)
    return float(np.mean(np.abs(a - b)))


def select_best_frames(
    frames: list[FrameInfo],
    sharpness_threshold: float,
    diff_threshold: float = 30.0,
) -> list[FrameInfo]:
    """Select the sharpest frame from each stable segment of consecutive frames."""
    if not frames:
        return []

    # Score sharpness for every frame
    scored: list[FrameInfo] = []
    for f in frames:
        s = compute_sharpness(f.path)
        scored.append(f._replace(sharpness=s))
        vlog(f"  Frame {f.index}: sharpness={s:.1f}")

    # Mark each frame as transitional or stable
    is_transitional: list[bool] = []
    for idx, f in enumerate(scored):
        blurry = f.sharpness < sharpness_threshold
        if idx > 0:
            diff = compute_frame_diff(scored[idx - 1].path, f.path)
            big_change = diff > diff_threshold
        else:
            big_change = False
        is_transitional.append(blurry or big_change)

    # Group consecutive stable frames into segments
    segments: list[list[FrameInfo]] = []
    current_segment: list[FrameInfo] = []
    for idx, f in enumerate(scored):
        if is_transitional[idx]:
            if current_segment:
                segments.append(current_segment)
                current_segment = []
        else:
            current_segment.append(f)
    if current_segment:
        segments.append(current_segment)

    # Pick the sharpest frame from each segment (discard segments < 2 frames)
    best: list[FrameInfo] = []
    for seg in segments:
        if len(seg) < 2:
            continue
        winner = max(seg, key=lambda f: f.sharpness)
        best.append(winner)

    log(f"  Selected {len(best)} frames from {len(segments)} segments")
    return best


# ── Stage 3: Page Deduplication ─────────────────────────────────────────────


def deduplicate_pages(
    frames: list[FrameInfo], hash_threshold: int
) -> list[FrameInfo]:
    """Remove duplicate pages using perceptual hashing (consecutive then global)."""
    if not frames:
        return []

    # Compute perceptual hashes
    hashes = []
    for f in frames:
        h = imagehash.phash(Image.open(f.path))
        hashes.append(h)

    # Pass 1: consecutive dedup -- keep the sharper of two consecutive matches
    pass1: list[int] = [0]  # indices into frames; always keep first
    for i in range(1, len(frames)):
        last_idx = pass1[-1]
        dist = hashes[i] - hashes[last_idx]
        if dist <= hash_threshold:
            # Same page -- keep the sharper one
            if frames[i].sharpness > frames[last_idx].sharpness:
                pass1[-1] = i
                vlog(
                    f"  Consecutive dup: frame {frames[i].index} replaces "
                    f"{frames[last_idx].index} (sharper, dist={dist})"
                )
            else:
                vlog(
                    f"  Consecutive dup: frame {frames[i].index} discarded "
                    f"(dist={dist})"
                )
        else:
            pass1.append(i)

    # Pass 2: global dedup -- discard revisits (keep first occurrence)
    seen_hashes: list[imagehash.ImageHash] = []
    pass2: list[int] = []
    for i in pass1:
        h = hashes[i]
        is_dup = False
        for seen in seen_hashes:
            if h - seen <= hash_threshold:
                is_dup = True
                vlog(f"  Global dup: frame {frames[i].index} discarded (revisit)")
                break
        if not is_dup:
            seen_hashes.append(h)
            pass2.append(i)

    unique = [frames[i] for i in pass2]
    log(f"  Deduplicated: {len(unique)} unique pages from {len(frames)} frames")
    return unique


# ── Stage 4: OCR ────────────────────────────────────────────────────────────

_LANG_MAP_TESSERACT = {"fr": "fra", "en": "eng"}

# Surya lazy-init globals
_surya_rec = None
_surya_det = None
_surya_layout = None


def _init_surya():
    """Lazy-initialize Surya predictors (downloads models on first call)."""
    global _surya_rec, _surya_det, _surya_layout
    if _surya_rec is not None:
        return
    vlog("  Loading Surya models (first run downloads ~1-2 GB)...")
    from surya.foundation import FoundationPredictor
    from surya.recognition import RecognitionPredictor
    from surya.detection import DetectionPredictor
    from surya.layout import LayoutPredictor

    foundation = FoundationPredictor(device="cpu")
    _surya_rec = RecognitionPredictor(foundation)
    _surya_det = DetectionPredictor()
    _surya_layout = LayoutPredictor(foundation)
    vlog("  Surya models loaded")


def _bbox_from_polygon(polygon) -> tuple[float, float, float, float]:
    """Convert a polygon (list of [x, y]) to (x1, y1, x2, y2) bbox."""
    xs = [p[0] for p in polygon]
    ys = [p[1] for p in polygon]
    return (min(xs), min(ys), max(xs), max(ys))


def _get_line_label(line_polygon, layout_bboxes) -> str:
    """Match an OCR line to a layout region by spatial overlap."""
    line_bbox = _bbox_from_polygon(line_polygon)
    lx1, ly1, lx2, ly2 = line_bbox
    line_cy = (ly1 + ly2) / 2

    best_label = "Text"
    best_overlap = 0.0

    for lb in layout_bboxes:
        bx1, by1, bx2, by2 = _bbox_from_polygon(lb.polygon)
        # Check if line center is inside layout box
        if bx1 <= (lx1 + lx2) / 2 <= bx2 and by1 <= line_cy <= by2:
            # Compute horizontal overlap
            ox = max(0, min(lx2, bx2) - max(lx1, bx1))
            line_w = max(lx2 - lx1, 1)
            overlap = ox / line_w
            if overlap > best_overlap:
                best_overlap = overlap
                best_label = lb.label
    return best_label


def _ocr_page_surya(image_path: str, lang: str, page_idx: int) -> tuple[list[OcrLine], object]:
    """OCR a single page with Surya. Returns (lines, layout_result)."""
    _init_surya()
    img = Image.open(image_path).convert("RGB")

    # Layout detection
    layout_result = _surya_layout([img])
    layout_bboxes = layout_result[0].bboxes if layout_result else []

    # OCR (det_predictor required for automatic text line detection)
    ocr_result = _surya_rec([img], det_predictor=_surya_det)

    lines: list[OcrLine] = []
    for text_line in ocr_result[0].text_lines:
        text = text_line.text.strip()
        if not text:
            continue
        bbox = _bbox_from_polygon(text_line.polygon)
        height = bbox[3] - bbox[1]
        confidence = text_line.confidence if text_line.confidence else 0.0
        label = _get_line_label(text_line.polygon, layout_bboxes)
        lines.append(OcrLine(
            text=text, bbox=bbox, height=height,
            confidence=confidence, page_idx=page_idx, label=label,
        ))

    return lines, layout_result[0] if layout_result else None


def _ocr_page_tesseract(image_path: str, lang: str, page_idx: int) -> list[OcrLine]:
    """OCR a single page with Tesseract. Returns lines."""
    import pytesseract

    tess_lang = _LANG_MAP_TESSERACT.get(lang, "eng")
    img = Image.open(image_path)

    # Get word-level data with bounding boxes
    data = pytesseract.image_to_data(img, lang=tess_lang, output_type=pytesseract.Output.DICT)

    # Group words by (block, par, line)
    line_groups: dict[tuple[int, int, int], list[dict]] = {}
    for i in range(len(data["text"])):
        text = data["text"][i].strip()
        conf = int(data["conf"][i])
        if conf < 0 or not text:
            continue
        key = (data["block_num"][i], data["par_num"][i], data["line_num"][i])
        if key not in line_groups:
            line_groups[key] = []
        line_groups[key].append({
            "text": text,
            "top": data["top"][i],
            "height": data["height"][i],
            "left": data["left"][i],
            "width": data["width"][i],
            "conf": conf,
        })

    lines: list[OcrLine] = []
    for key in sorted(line_groups.keys()):
        words = line_groups[key]
        text = " ".join(w["text"] for w in words)
        median_h = float(np.median([w["height"] for w in words]))
        mean_top = float(np.mean([w["top"] for w in words]))
        mean_conf = float(np.mean([w["conf"] for w in words])) / 100.0
        x1 = min(w["left"] for w in words)
        x2 = max(w["left"] + w["width"] for w in words)
        y1 = mean_top
        y2 = mean_top + median_h

        lines.append(OcrLine(
            text=text, bbox=(x1, y1, x2, y2), height=median_h,
            confidence=mean_conf, page_idx=page_idx, label="Text",
        ))

    return lines


def ocr_all_pages(
    pages: list[FrameInfo], lang: str
) -> tuple[list[list[OcrLine]], list]:
    """OCR all pages. Returns (lines_per_page, layout_results_per_page)."""
    all_lines: list[list[OcrLine]] = []
    all_layouts: list = []

    for i, page in enumerate(pages):
        vlog(f"  OCR page {i + 1}/{len(pages)}: {os.path.basename(page.path)}")
        if Config.ocr_engine == "surya":
            lines, layout = _ocr_page_surya(page.path, lang, i)
            all_lines.append(lines)
            all_layouts.append(layout)
        else:
            lines = _ocr_page_tesseract(page.path, lang, i)
            all_lines.append(lines)
            all_layouts.append(None)

    total_lines = sum(len(ll) for ll in all_lines)
    log(f"  OCR complete: {total_lines} lines across {len(pages)} pages")
    return all_lines, all_layouts


# ── Stage 5: Section Splitting ──────────────────────────────────────────────

HEADER_FOOTER_MARGIN = 0.05
SECTION_NUM_RE = re.compile(r'^\s*(\d{1,3})\s*$')


def _get_page_height(page_lines: list[OcrLine]) -> float:
    """Estimate page height from OCR line positions."""
    if not page_lines:
        return 1000.0
    return max(line.bbox[3] for line in page_lines)


def _is_section_header(
    line: OcrLine, page_height: float
) -> int | None:
    """Return section number if line is a section header, else None.

    In gamebooks, section numbers appear as bare standalone numbers (e.g. "368")
    on their own line. In photographed book spreads, height-based detection is
    unreliable because perspective distortion makes body text lines taller than
    section numbers. So the primary signal is simply: a line that is ONLY a number.
    """
    m = SECTION_NUM_RE.match(line.text)
    if not m:
        return None

    # Exclude page headers/footers (top/bottom 5%)
    y_center = (line.bbox[1] + line.bbox[3]) / 2
    if y_center < page_height * HEADER_FOOTER_MARGIN:
        return None
    if y_center > page_height * (1 - HEADER_FOOTER_MARGIN):
        return None

    # A standalone bare number is a section header
    return int(m.group(1))


def split_into_sections(
    all_lines: list[list[OcrLine]], all_layouts: list
) -> list[Section]:
    """Split OCR lines into numbered sections based on standalone section numbers."""
    if not any(all_lines):
        log("  WARNING: No OCR text found on any page")
        return []

    # Walk all pages and detect section boundaries
    sections: list[Section] = []
    current_num: int | None = None
    current_lines: list[str] = []
    current_start_page = 0

    for page_idx, page_lines in enumerate(all_lines):
        page_height = _get_page_height(page_lines)
        for line in page_lines:
            sec_num = _is_section_header(line, page_height)
            if sec_num is not None:
                # Flush previous section
                if current_num is not None:
                    sections.append(Section(
                        number=current_num,
                        lines=current_lines,
                        images=[],
                        page_range=(current_start_page, page_idx),
                    ))
                current_num = sec_num
                current_lines = []
                current_start_page = page_idx
                vlog(f"  Section § {sec_num} starts on page {page_idx + 1}")
            elif current_num is not None:
                # Skip lines that are part of image regions
                if line.label in ("Picture", "Figure", "Table"):
                    continue
                current_lines.append(line.text)

    # Flush final section
    if current_num is not None:
        sections.append(Section(
            number=current_num,
            lines=current_lines,
            images=[],
            page_range=(current_start_page, len(all_lines) - 1),
        ))

    # Sort by section number
    sections.sort(key=lambda s: s.number)

    # Warn on duplicates — keep the one with more text
    seen: dict[int, int] = {}
    deduped: list[Section] = []
    for sec in sections:
        if sec.number in seen:
            idx = seen[sec.number]
            existing = deduped[idx]
            if len(sec.lines) > len(existing.lines):
                vlog(f"  Duplicate § {sec.number}: keeping longer version")
                deduped[idx] = sec
            else:
                vlog(f"  Duplicate § {sec.number}: keeping existing version")
        else:
            seen[sec.number] = len(deduped)
            deduped.append(sec)

    log(f"  {len(deduped)} sections found (§ {deduped[0].number}–{deduped[-1].number})" if deduped else "  No sections found")
    return deduped


# ── Stage 6: Image Extraction ──────────────────────────────────────────────


def _detect_image_regions_surya(layout_result) -> list[tuple[int, int, int, int]]:
    """Extract image region bboxes from Surya layout result."""
    regions = []
    if layout_result is None:
        return regions
    for bbox in layout_result.bboxes:
        if bbox.label in ("Picture", "Figure"):
            b = _bbox_from_polygon(bbox.polygon)
            regions.append((int(b[0]), int(b[1]), int(b[2]), int(b[3])))
    return regions


def _detect_image_regions_opencv(image_path: str) -> list[tuple[int, int, int, int]]:
    """Fallback image region detection using OpenCV contours."""
    img = cv2.imread(image_path)
    if img is None:
        return []
    gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
    h, w = gray.shape

    # Threshold to find non-text (dark/complex) regions
    blurred = cv2.GaussianBlur(gray, (5, 5), 0)
    edges = cv2.Canny(blurred, 30, 100)
    dilated = cv2.dilate(edges, None, iterations=3)

    contours, _ = cv2.findContours(dilated, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)

    regions = []
    min_area = h * w * 0.02  # at least 2% of page
    max_area = h * w * 0.80  # at most 80% of page
    for cnt in contours:
        x, y, cw, ch = cv2.boundingRect(cnt)
        area = cw * ch
        if min_area < area < max_area:
            aspect = cw / max(ch, 1)
            # Images tend to be roughly square-ish or wider, not extremely tall/narrow
            if 0.2 < aspect < 5.0:
                regions.append((x, y, x + cw, y + ch))

    return regions


def _find_owning_section(
    img_y_center: float,
    page_idx: int,
    sections: list[Section],
    header_positions: dict[int, list[tuple[int, float]]],
) -> int | None:
    """Find which section an image belongs to based on vertical position."""
    headers = header_positions.get(page_idx, [])

    if headers:
        # Find nearest header above the image
        candidates = [(num, y) for num, y in headers if y <= img_y_center]
        if candidates:
            return max(candidates, key=lambda c: c[1])[0]

    # No header above on this page — find section that owns this page
    for sec in sections:
        if sec.page_range[0] <= page_idx <= sec.page_range[1]:
            return sec.number

    return None


def extract_images(
    pages: list[FrameInfo],
    all_layouts: list,
    all_lines: list[list[OcrLine]],
    sections: list[Section],
    images_dir: str,
    ref_pages: int,
) -> tuple[list[Section], list[str]]:
    """Extract images from pages and associate with sections.

    Returns (updated_sections, ref_image_paths).
    """
    ref_image_paths: list[str] = []
    os.makedirs(images_dir, exist_ok=True)

    # Save reference pages (first ref_pages pages as full-page images)
    for i in range(min(ref_pages, len(pages))):
        src = pages[i].path
        dst = os.path.join(images_dir, f"ref_{i + 1:03d}.jpg")
        img = cv2.imread(src)
        if img is not None:
            cv2.imwrite(dst, img, [cv2.IMWRITE_JPEG_QUALITY, 90])
            ref_image_paths.append(dst)
            vlog(f"  Reference page {i + 1} saved")

    # Build header position map from OCR data for accurate image-to-section association
    header_positions: dict[int, list[tuple[int, float]]] = {}
    for page_idx, page_lines in enumerate(all_lines):
        page_height = max((l.bbox[3] for l in page_lines), default=1000.0)
        for line in page_lines:
            sec_num = _is_section_header(line, page_height)
            if sec_num is not None:
                if page_idx not in header_positions:
                    header_positions[page_idx] = []
                y_center = (line.bbox[1] + line.bbox[3]) / 2
                header_positions[page_idx].append((sec_num, y_center))

    # Extract images from non-reference pages
    section_images: dict[int, list[str]] = {s.number: list(s.images) for s in sections}
    fig_counter = 0

    for page_idx in range(ref_pages, len(pages)):
        page = pages[page_idx]
        layout = all_layouts[page_idx] if page_idx < len(all_layouts) else None

        if Config.ocr_engine == "surya" and layout is not None:
            regions = _detect_image_regions_surya(layout)
        else:
            regions = _detect_image_regions_opencv(page.path)

        if not regions:
            continue

        img = cv2.imread(page.path)
        if img is None:
            continue
        h, w = img.shape[:2]

        for x1, y1, x2, y2 in regions:
            # Clamp to image bounds
            x1, y1 = max(0, x1), max(0, y1)
            x2, y2 = min(w, x2), min(h, y2)
            if x2 - x1 < 20 or y2 - y1 < 20:
                continue

            fig_counter += 1
            crop = img[y1:y2, x1:x2]
            img_y_center = (y1 + y2) / 2.0

            owner = _find_owning_section(img_y_center, page_idx, sections, header_positions)

            if owner is not None:
                fname = f"sec{owner:03d}_fig{fig_counter}.jpg"
            else:
                fname = f"page{page_idx:03d}_fig{fig_counter}.jpg"

            dst = os.path.join(images_dir, fname)
            cv2.imwrite(dst, crop, [cv2.IMWRITE_JPEG_QUALITY, 90])

            if owner is not None and owner in section_images:
                section_images[owner].append(os.path.join("images", fname))
                vlog(f"  Image {fname} → § {owner}")
            else:
                vlog(f"  Image {fname} → unassociated")

    # Rebuild sections with updated image lists
    updated = []
    for sec in sections:
        updated.append(Section(
            number=sec.number,
            lines=sec.lines,
            images=section_images.get(sec.number, []),
            page_range=sec.page_range,
        ))

    log(f"  {fig_counter} images extracted, {len(ref_image_paths)} reference pages")
    return updated, ref_image_paths


# ── Stage 7: LLM Cleanup ───────────────────────────────────────────────────

_SYSTEM_PROMPT_FR = """\
You are an OCR post-processor for a French gamebook (livre dont vous êtes le héros).
Fix these OCR artifacts:
- Broken words split across lines: rejoin them
- Missing French accents: restore é, è, ê, ë, à, â, ç, ù, û, î, ï, ô, œ, æ
- Ligature restoration: oeuvre→œuvre, coeur→cœur, soeur→sœur
- OCR noise: remove isolated characters, garbled fragments
- Fix common OCR confusions: rn→m, cl→d, I→l where contextually appropriate

Rules:
- Preserve section cross-references exactly (e.g., "rendez-vous au 147", "allez au 32")
- Do NOT change section numbers
- Do NOT add or remove content — only fix OCR errors
- Do NOT translate — keep all text in French
- Output ONLY the cleaned text, no commentary or explanation"""

_SYSTEM_PROMPT_EN = """\
You are an OCR post-processor for an English gamebook (Fighting Fantasy / Choose Your Own Adventure).
Fix these OCR artifacts:
- Broken words split across lines: rejoin them
- OCR noise: remove isolated characters, garbled fragments
- Fix common OCR confusions: rn→m, cl→d, I→l where contextually appropriate

Rules:
- Preserve section cross-references exactly (e.g., "go to 147", "turn to 32")
- Do NOT change section numbers
- Do NOT add or remove content — only fix OCR errors
- Output ONLY the cleaned text, no commentary or explanation"""


def _call_claude(section_text: str, system_prompt: str) -> str:
    """Call the Claude CLI to clean a section's text. Returns original on failure."""
    try:
        result = subprocess.run(
            ["claude", "--print", "--model", "sonnet",
             "--max-turns", "1",
             "--system-prompt", system_prompt],
            input=section_text,
            capture_output=True,
            text=True,
            timeout=300,
        )
        if result.returncode != 0:
            vlog(f"  Claude CLI returned exit code {result.returncode}")
            return section_text
        cleaned = result.stdout.strip()
        if not cleaned:
            return section_text
        return cleaned
    except FileNotFoundError:
        return section_text
    except subprocess.TimeoutExpired:
        vlog("  Claude CLI timed out")
        return section_text
    except Exception as e:
        vlog(f"  Claude CLI error: {e}")
        return section_text


def llm_cleanup_sections(sections: list[Section], lang: str) -> list[Section]:
    """Clean OCR text via Claude CLI."""
    # Check if claude CLI is available
    if not shutil.which("claude"):
        log("  WARNING: claude CLI not found — skipping LLM cleanup")
        log("  Install from: https://claude.ai/code")
        return sections

    system_prompt = _SYSTEM_PROMPT_FR if lang == "fr" else _SYSTEM_PROMPT_EN
    cleaned: list[Section] = []

    for i, sec in enumerate(sections):
        text = "\n".join(sec.lines)
        if not text.strip():
            cleaned.append(sec)
            continue

        vlog(f"  Cleaning § {sec.number} ({i + 1}/{len(sections)})...")
        result = _call_claude(text, system_prompt)
        new_lines = result.split("\n")
        cleaned.append(Section(
            number=sec.number,
            lines=new_lines,
            images=sec.images,
            page_range=sec.page_range,
        ))

    log(f"  LLM cleanup complete: {len(cleaned)} sections processed")
    return cleaned


# ── Stage 8: Markdown Assembly ──────────────────────────────────────────────


def assemble_markdown(
    sections: list[Section],
    ref_images: list[str],
    title: str,
    lang: str,
    ref_pages: int,
    output_path: str,
) -> None:
    """Write sections and reference images to a gamebook markdown file."""
    lines: list[str] = []
    output_dir = os.path.dirname(output_path)

    # YAML frontmatter
    lines.append("---")
    lines.append(f"title: {title}")
    lines.append(f"lang: {lang}")
    lines.append(f"ref_pages: {ref_pages}")
    lines.append("---")
    lines.append("")

    # Reference images
    for i, ref_path in enumerate(ref_images):
        rel_path = os.path.relpath(ref_path, output_dir)
        lines.append(f"<!-- REF: Page {i + 1} -->")
        lines.append(f"![Reference Page {i + 1}]({rel_path})")
        lines.append("")

    if ref_images:
        lines.append("---")
        lines.append("")

    # Sections
    for sec_idx, sec in enumerate(sections):
        lines.append(f"## § {sec.number}")
        lines.append("")

        # Section body text
        text = "\n".join(sec.lines).strip()
        if text:
            lines.append(text)
            lines.append("")

        # Section images
        for img_path in sec.images:
            # img_path is relative like "images/sec001_fig1.jpg"
            lines.append(f"![]({img_path})")
            lines.append("")

        # Section separator
        if sec_idx < len(sections) - 1:
            lines.append("---")
            lines.append("")

    # Write file
    with open(output_path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))

    log(f"  Markdown written: {output_path} ({len(sections)} sections)")


# ── CLI ─────────────────────────────────────────────────────────────────────


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="gamebook-digitize",
        description="Convert a gamebook video to markdown + interactive HTML player.",
    )
    parser.add_argument("input", nargs="?", help="Video file path")
    parser.add_argument(
        "--from-markdown",
        metavar="PATH",
        help="Skip video pipeline, generate HTML from existing markdown file",
    )
    parser.add_argument(
        "-l",
        "--lang",
        default="fr",
        choices=["fr", "en"],
        help="Book language (default: fr)",
    )
    parser.add_argument(
        "--ref-pages",
        type=int,
        default=0,
        help="Number of initial pages to treat as reference material",
    )
    parser.add_argument(
        "-o",
        "--output",
        metavar="DIR",
        help="Output directory (default: ./<input-stem>/)",
    )
    parser.add_argument(
        "--ocr-engine",
        choices=["surya", "tesseract"],
        default="surya",
        help="OCR engine (default: surya)",
    )
    parser.add_argument(
        "--no-llm",
        action="store_true",
        help="Skip Claude CLI cleanup pass",
    )
    parser.add_argument(
        "--frame-interval",
        type=float,
        default=0.5,
        help="Seconds between extracted frames (default: 0.5)",
    )
    parser.add_argument(
        "--sharpness-threshold",
        type=float,
        default=50.0,
        help="Laplacian variance below which = blurry (default: 50.0)",
    )
    parser.add_argument(
        "--hash-threshold",
        type=int,
        default=8,
        help="Max hamming distance for 'same page' dedup (default: 8)",
    )
    parser.add_argument(
        "--keep-frames",
        action="store_true",
        help="Save selected page images to output/frames/",
    )
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Detailed progress to stderr",
    )
    parser.add_argument(
        "--title",
        help="Book title (default: auto-detect from first page or 'Gamebook')",
    )

    args = parser.parse_args()

    # Validation
    if args.from_markdown:
        if not os.path.isfile(args.from_markdown):
            parser.error(f"Markdown file not found: {args.from_markdown}")
    else:
        if args.input is None:
            parser.error(
                "A video file is required (or use --from-markdown PATH)"
            )
        if not os.path.isfile(args.input):
            parser.error(f"Video file not found: {args.input}")

    return args


# ── Main ────────────────────────────────────────────────────────────────────


def main() -> None:
    args = parse_args()
    Config.verbose = args.verbose
    Config.ocr_engine = args.ocr_engine
    Config.no_llm = args.no_llm
    Config.ref_pages = args.ref_pages

    # --from-markdown mode: skip video pipeline
    if args.from_markdown:
        from html_generator import main_standalone

        md_path = args.from_markdown
        output_dir = args.output or str(Path(md_path).parent)
        html_path = os.path.join(output_dir, "player.html")
        main_standalone(md_path, html_path)
        return

    # Full pipeline mode
    video_path = args.input
    output_dir = args.output or Path(video_path).stem
    os.makedirs(output_dir, exist_ok=True)
    images_dir = os.path.join(output_dir, "images")
    os.makedirs(images_dir, exist_ok=True)
    md_path = os.path.join(output_dir, "sections.md")
    html_path = os.path.join(output_dir, "player.html")
    title = args.title or "Gamebook"

    with tempfile.TemporaryDirectory(prefix="gamebook-") as tmpdir:
        log("Stage 1/9: Extracting frames...")
        frames = extract_frames(video_path, tmpdir, args.frame_interval)
        log(f"  {len(frames)} frames extracted")

        log("Stage 2/9: Selecting best frames...")
        best = select_best_frames(frames, args.sharpness_threshold)
        log(f"  {len(best)} frames selected")

        log("Stage 3/9: Deduplicating pages...")
        pages = deduplicate_pages(best, args.hash_threshold)
        log(f"  {len(pages)} unique pages")

        if args.keep_frames:
            frames_dir = os.path.join(output_dir, "frames")
            os.makedirs(frames_dir, exist_ok=True)
            for i, p in enumerate(pages):
                shutil.copy2(p.path, os.path.join(frames_dir, f"page_{i:04d}.jpg"))
            log(f"  Saved page images to {frames_dir}/")

        log("Stage 4/9: Running OCR...")
        all_lines, all_layouts = ocr_all_pages(pages, args.lang)

        log("Stage 5/9: Splitting into sections...")
        # Exclude reference pages from section splitting (offset page indices back)
        rp = args.ref_pages
        sections_raw = split_into_sections(all_lines[rp:], all_layouts[rp:])
        sections = [
            Section(s.number, s.lines, s.images,
                    (s.page_range[0] + rp, s.page_range[1] + rp))
            for s in sections_raw
        ]
        log(f"  {len(sections)} sections found")

        log("Stage 6/9: Extracting images...")
        sections, ref_images = extract_images(
            pages, all_layouts, all_lines, sections, images_dir, args.ref_pages
        )

        if not args.no_llm:
            log("Stage 7/9: LLM cleanup...")
            sections = llm_cleanup_sections(sections, args.lang)
        else:
            log("Stage 7/9: LLM cleanup (skipped)")

        log("Stage 8/9: Assembling markdown...")
        assemble_markdown(
            sections, ref_images, title, args.lang, args.ref_pages, md_path
        )

        log("Stage 9/9: Generating HTML player...")
        from html_generator import generate_from_markdown

        generate_from_markdown(md_path, html_path)

    log(f"Done! {len(sections)} sections extracted.")
    log(f"  Markdown: {md_path}")
    log(f"  HTML:     {html_path}")


if __name__ == "__main__":
    main()
