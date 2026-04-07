#!/usr/bin/env python3
"""
whisper_transcribe.py — Transcribe a WAV file using faster-whisper.

Called by the Rust binary as a subprocess:

    python3 whisper_transcribe.py <wav_path> <model_name> [language]

Prints the transcript to stdout (and nothing else — the Rust side reads
stdout and injects it as keystrokes).

All diagnostic messages go to stderr so they don't pollute stdout.

DEPENDENCIES
────────────
Install once with:

    pip install faster-whisper --break-system-packages

faster-whisper uses CTranslate2 under the hood — much faster than the
original openai-whisper on CPU because it uses 8-bit integer quantisation.
"""

from __future__ import annotations

import sys
import os


# ─────────────────────────────────────────────────────────────────────────────
# Argument parsing (manual, no argparse, to keep the import budget tiny)
# ─────────────────────────────────────────────────────────────────────────────

def parse_args() -> tuple[str, str, str]:
    """
    Expect two required positional arguments and one optional:
        sys.argv[1] → path to the WAV file
        sys.argv[2] → Whisper model name (e.g. "base", "small", "medium")
        sys.argv[3] → language code (e.g. "en", "fr") — defaults to "en"
    """
    if len(sys.argv) < 3:
        print(
            "Usage: whisper_transcribe.py <wav_path> <model_name> [language]",
            file=sys.stderr,
        )
        sys.exit(1)

    wav_path   = sys.argv[1]
    model_name = sys.argv[2]
    language   = sys.argv[3] if len(sys.argv) > 3 else "en"

    valid_languages = ("en", "fr")
    if language not in valid_languages:
        print(
            f"ERROR: invalid language '{language}'. Must be one of: {', '.join(valid_languages)}",
            file=sys.stderr,
        )
        sys.exit(1)

    if not os.path.isfile(wav_path):
        print(f"ERROR: WAV file not found: {wav_path}", file=sys.stderr)
        sys.exit(1)

    return wav_path, model_name, language


# ─────────────────────────────────────────────────────────────────────────────
# Model loading
# ─────────────────────────────────────────────────────────────────────────────

# We cache the model in a module-level variable.  When Rust calls us as a
# subprocess this cache is always empty (new process each time), but it is
# here so the script can also be used interactively or in a long-running
# server variant.
_MODEL_CACHE: dict = {}


def load_model(model_name: str):
    """
    Load (or retrieve from cache) a faster-whisper WhisperModel.

    Parameters
    ----------
    model_name:
        One of "tiny", "base", "small", "medium", "large-v3", etc.
        Smaller models are faster; "base" is the sweet spot for CPU.

    Returns
    -------
    A WhisperModel instance ready for transcription.
    """
    if model_name in _MODEL_CACHE:
        return _MODEL_CACHE[model_name]

    try:
        from faster_whisper import WhisperModel
    except ImportError:
        print(
            "ERROR: faster-whisper is not installed.\n"
            "Run:  pip install faster-whisper --break-system-packages",
            file=sys.stderr,
        )
        sys.exit(1)

    print(f"[whisper] Loading model '{model_name}' (first run may download it)…",
          file=sys.stderr)

    # device="cpu"  → use CPU (no ROCm/CUDA needed, perfect for your setup)
    # compute_type  → "int8" is the fastest CPU mode with minimal accuracy loss
    #                 use "float16" if you ever add a supported GPU
    model = WhisperModel(
        model_name,
        device="cpu",
        compute_type="int8",
    )

    _MODEL_CACHE[model_name] = model
    return model


# ─────────────────────────────────────────────────────────────────────────────
# Transcription
# ─────────────────────────────────────────────────────────────────────────────

def transcribe(wav_path: str, model_name: str, language: str = "en") -> str:
    """
    Run Whisper on the given WAV file and return the full transcript.

    faster-whisper returns an iterable of Segment objects, each with a `.text`
    attribute.  We join them with a single space.

    Parameters
    ----------
    wav_path:
        Absolute or relative path to a 16-bit PCM WAV file.
    model_name:
        Whisper model size string.
    language:
        Language code for speech recognition (e.g. "en", "fr").

    Returns
    -------
    The full transcript as a single string (may be empty if silence detected).
    """
    model = load_model(model_name)

    # `transcribe` returns (segments_generator, TranscriptionInfo).
    # We iterate segments lazily — no need to load them all into memory.
    #
    # vad_filter=True  → use the built-in Silero VAD to skip silent regions.
    #                    This prevents Whisper from hallucinating words in silence.
    # language         → passed from CLI (e.g. "en", "fr"); forces the language
    #                    instead of auto-detecting, which is faster and more reliable.
    segments, info = model.transcribe(
        wav_path,
        vad_filter=True,        # skip silence (Silero VAD built into faster-whisper)
        language=language,       # "en", "fr", etc. — passed from CLI
        beam_size=5,             # higher = more accurate, slower. 5 is the default.
        best_of=5,               # number of candidates (greedy decode if 1)
        temperature=0.0,         # deterministic output; raise if results feel flat
    )

    print(
        f"[whisper] Detected language '{info.language}' "
        f"(confidence {info.language_probability:.0%})",
        file=sys.stderr,
    )

    # Collect all segment texts and join them.
    # Strip leading/trailing whitespace from each segment before joining.
    parts: list[str] = []
    for segment in segments:
        text = segment.text.strip()
        if text:
            parts.append(text)
            print(f"[whisper] Segment [{segment.start:.1f}s → {segment.end:.1f}s]: {text}",
                  file=sys.stderr)

    transcript = " ".join(parts)
    return transcript


# ─────────────────────────────────────────────────────────────────────────────
# Entry point
# ─────────────────────────────────────────────────────────────────────────────

def main() -> None:
    wav_path, model_name, language = parse_args()

    transcript = transcribe(wav_path, model_name, language)

    # Print ONLY the transcript to stdout.
    # The Rust binary reads this and types it via xdotool.
    print(transcript, end="")   # no trailing newline — we don't want an Enter keystroke


if __name__ == "__main__":
    main()
