#!/usr/bin/env python3
"""
tts_speak.py — Text-to-speech using Piper (ONNX, CPU, fully offline).

Called by the Rust binary as a subprocess:

    python3 tts_speak.py "<text>" <voice> <speed>

Synthesizes speech and plays it through the default audio output device.
All diagnostic messages go to stderr.

DEPENDENCIES
────────────
Install once with:

    pip install piper-tts sounddevice numpy
"""

from __future__ import annotations

import sys

# ─────────────────────────────────────────────────────────────────────────────
# Argument parsing
# ─────────────────────────────────────────────────────────────────────────────

def parse_args() -> tuple[str, str, float]:
    """
    Expect positional arguments:
        sys.argv[1] → text to speak
        sys.argv[2] → Piper voice model name (e.g. "en_US-lessac-medium")
        sys.argv[3] → speed multiplier (float, default 1.0)
    """
    if len(sys.argv) < 3:
        print(
            "Usage: tts_speak.py <text> <voice> [speed]",
            file=sys.stderr,
        )
        sys.exit(1)

    text = sys.argv[1]
    voice = sys.argv[2]
    speed = float(sys.argv[3]) if len(sys.argv) >= 4 else 1.0

    if not text.strip():
        print("WARNING: Empty text, nothing to speak.", file=sys.stderr)
        sys.exit(0)

    return text, voice, speed


# ─────────────────────────────────────────────────────────────────────────────
# Model directory
# ─────────────────────────────────────────────────────────────────────────────

def get_model_dir() -> str:
    """Return the path where Piper models are stored."""
    import os
    return os.path.join(
        os.environ.get("XDG_DATA_HOME", os.path.expanduser("~/.local/share")),
        "voice-speak", "models",
    )


# ─────────────────────────────────────────────────────────────────────────────
# TTS synthesis + playback
# ─────────────────────────────────────────────────────────────────────────────

def speak(text: str, voice: str, speed: float) -> None:
    """
    Synthesize speech with Piper and play it through the default output device.

    Parameters
    ----------
    text:
        The text to speak aloud.
    voice:
        Piper voice model name (e.g. "en_US-lessac-medium").
    speed:
        Speech rate multiplier.  1.0 = normal speed, 2.0 = double speed.
    """
    try:
        import piper
        from piper.config import SynthesisConfig
    except ImportError:
        print(
            "ERROR: piper-tts is not installed.\n"
            "Run:  pip install piper-tts",
            file=sys.stderr,
        )
        sys.exit(1)

    import numpy as np
    import os
    import glob
    import subprocess
    import tempfile
    import wave

    model_dir = get_model_dir()

    # Find the .onnx model file in the model directory.
    model_pattern = os.path.join(model_dir, f"{voice}*", f"{voice}*.onnx")
    model_files = glob.glob(model_pattern)

    # Also try flat layout: models/<voice>.onnx
    if not model_files:
        model_pattern = os.path.join(model_dir, f"{voice}.onnx")
        model_files = glob.glob(model_pattern)

    if not model_files:
        print(
            f"ERROR: No Piper model found for '{voice}' in {model_dir}\n"
            f"Run ./install.sh to download all language models.",
            file=sys.stderr,
        )
        sys.exit(1)

    model_path = model_files[0]
    print(f"[tts] Loading model: {model_path}", file=sys.stderr)

    voice_obj = piper.PiperVoice.load(model_path)

    print(f"[tts] Synthesizing {len(text)} chars (speed={speed})…", file=sys.stderr)

    # length_scale < 1.0 = faster, > 1.0 = slower.  We invert the user's
    # speed multiplier so that speed=2.0 means "twice as fast".
    syn_config = SynthesisConfig(length_scale=1.0 / speed)

    # Piper's synthesize() yields AudioChunk objects (one per sentence).
    chunks = list(voice_obj.synthesize(text, syn_config=syn_config))

    if not chunks:
        print("[tts] No audio generated.", file=sys.stderr)
        return

    sample_rate = chunks[0].sample_rate
    audio = np.concatenate([chunk.audio_float_array for chunk in chunks])

    # Convert float32 [-1.0, 1.0] → int16 PCM for WAV.
    audio_i16 = (audio.clip(-1.0, 1.0) * 32767).astype(np.int16)

    # Write to a temp WAV file and play via paplay (PulseAudio/PipeWire).
    # This ensures audio goes through the system sound server and reaches
    # the correct output device, unlike raw ALSA which can be silent.
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=True) as tmp:
        with wave.open(tmp.name, "wb") as wf:
            wf.setnchannels(1)
            wf.setsampwidth(2)  # 16-bit
            wf.setframerate(sample_rate)
            wf.writeframes(audio_i16.tobytes())

        duration = len(audio) / sample_rate
        print(f"[tts] Playing {duration:.1f}s of audio at {sample_rate} Hz", file=sys.stderr)

        # paplay routes through PulseAudio/PipeWire → correct output sink.
        result = subprocess.run(["paplay", tmp.name])
        if result.returncode != 0:
            print(f"[tts] paplay failed (exit {result.returncode})", file=sys.stderr)
            sys.exit(1)

    print("[tts] Done.", file=sys.stderr)


# ─────────────────────────────────────────────────────────────────────────────
# Entry point
# ─────────────────────────────────────────────────────────────────────────────

def main() -> None:
    text, voice, speed = parse_args()
    speak(text, voice, speed)


if __name__ == "__main__":
    main()
