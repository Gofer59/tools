from __future__ import annotations

import logging
import threading

import numpy as np

log = logging.getLogger("math_speak.audio")

_play_lock = threading.Lock()


def play(pcm_bytes: bytes, sample_rate: int) -> None:
    """Play raw 16-bit signed LE PCM mono. Blocking."""
    if not pcm_bytes:
        return
    try:
        import sounddevice as sd
    except (ImportError, OSError) as e:
        log.warning("sounddevice unavailable: %s", e)
        return
    samples = np.frombuffer(pcm_bytes, dtype=np.int16)
    with _play_lock:
        try:
            sd.play(samples, samplerate=sample_rate, blocking=True)
        except Exception as e:
            log.warning("playback failed: %s", e)
