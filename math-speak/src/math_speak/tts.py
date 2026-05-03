from __future__ import annotations

import io
import logging
import shutil
import subprocess
import wave
from pathlib import Path
from typing import Any

from .config import Config

log = logging.getLogger("math_speak.tts")

_voice_cache: dict[str, Any] = {}


def _find_onnx(model_dir: Path, voice: str) -> Path | None:
    cands = [
        model_dir / f"{voice}.onnx",
        *list(model_dir.glob(f"**/{voice}.onnx")),
    ]
    for c in cands:
        if c.is_file():
            return c
    return None


def _load_piper_voice(voice_name: str, model_dir: Path) -> Any:
    if voice_name in _voice_cache:
        return _voice_cache[voice_name]
    try:
        from piper import PiperVoice  # type: ignore
    except ImportError as e:
        log.warning("piper-tts not importable: %s", e)
        return None
    onnx = _find_onnx(model_dir, voice_name)
    if onnx is None:
        log.warning("piper voice not found: %s in %s", voice_name, model_dir)
        return None
    try:
        v = PiperVoice.load(str(onnx))
    except Exception as e:
        log.warning("PiperVoice.load failed: %s", e)
        return None
    _voice_cache[voice_name] = v
    return v


def synth_piper(text: str, cfg: Config) -> tuple[bytes, int] | None:
    """Return (pcm_int16_le_bytes, sample_rate) or None on failure."""
    voice = _load_piper_voice(cfg.voice_for_language(), cfg.expanded_model_dir())
    if voice is None:
        return None
    buf = io.BytesIO()
    try:
        with wave.open(buf, "wb") as wf:
            voice.synthesize_wav(text, wf)
    except Exception as e:
        log.warning("piper synth failed: %s", e)
        return None
    buf.seek(0)
    with wave.open(buf, "rb") as wf:
        sr = wf.getframerate()
        frames = wf.readframes(wf.getnframes())
    return frames, sr


def synth_espeak(text: str, lang: str) -> tuple[bytes, int] | None:
    """Run espeak-ng, return (pcm_int16_le, sample_rate=22050)."""
    if shutil.which("espeak-ng") is None:
        return None
    voice = "fr" if lang == "fr" else "en-us"
    try:
        proc = subprocess.run(
            ["espeak-ng", "-v", voice, "-s", "175", "--stdout", text],
            capture_output=True,
            timeout=20,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return None
    if proc.returncode != 0 or not proc.stdout:
        return None
    # espeak-ng outputs RIFF WAV
    try:
        with wave.open(io.BytesIO(proc.stdout), "rb") as wf:
            sr = wf.getframerate()
            frames = wf.readframes(wf.getnframes())
        return frames, sr
    except Exception as e:
        log.warning("espeak wav parse failed: %s", e)
        return None


def synthesize(spoken: str, engine_hint: str, cfg: Config) -> tuple[bytes, int] | None:
    if not spoken.strip():
        return None
    if engine_hint == "espeak":
        out = synth_espeak(spoken, cfg.language)
        if out:
            return out
    out = synth_piper(spoken, cfg)
    if out:
        return out
    if cfg.espeak_fallback:
        return synth_espeak(spoken, cfg.language)
    return None
