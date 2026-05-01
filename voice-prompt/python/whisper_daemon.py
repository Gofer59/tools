#!/usr/bin/env python3
"""Persistent Whisper daemon: stdin/stdout NDJSON IPC.

Two protocols served from one process:

1. Single-shot ``transcribe`` — used by the large (final) daemon. Unchanged.
2. Streaming ``stream_start`` / ``stream_chunk`` / ``stream_stop`` — used by
   the tiny (live-preview) daemon. Each ``stream_chunk`` request transcribes
   the WAV at the path provided and emits one ``{"event":"partial", ...}``
   line. ``stream_stop`` emits ``{"event":"final", ...}`` then
   ``{"status":"idle"}``.

Daemon is stateless across chunks (Rust owns the audio ring).
"""
import json
import os
import sys
import time

from faster_whisper import WhisperModel


def log(msg):
    print(msg, file=sys.stderr, flush=True)


def emit(obj):
    print(json.dumps(obj), flush=True)


def transcribe_once(model, wav, language, vad, beam_size=5, best_of=5):
    t0 = time.time()
    segments, info = model.transcribe(
        wav,
        vad_filter=vad,
        language=language,
        beam_size=beam_size,
        best_of=best_of,
        temperature=0.0,
    )
    text = " ".join(s.text.strip() for s in segments if s.text.strip())
    dt_ms = int((time.time() - t0) * 1000)
    return text, dt_ms, info


def main():
    if len(sys.argv) < 4:
        emit({"status": "error", "message": "usage: model compute_type model_dir [device]"})
        sys.exit(1)

    model_name, compute_type, model_dir = sys.argv[1], sys.argv[2], sys.argv[3]
    device = sys.argv[4] if len(sys.argv) > 4 else "cpu"
    log(f"[whisperd] loading {model_name} compute={compute_type} device={device} dir={model_dir}")

    model = WhisperModel(
        model_size_or_path=model_name,
        device=device,
        compute_type=compute_type,
        download_root=model_dir,
    )

    emit({"status": "ready", "model": model_name})

    streaming = False
    stream_lang = "en"
    stream_vad = True
    last_partial_text = ""

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
            cmd = req.get("cmd")

            if cmd == "quit":
                emit({"status": "quitting"})
                return

            if cmd == "transcribe":
                wav = req["wav"]
                language = req.get("language", "en")
                if language == "auto":
                    language = None
                vad = req.get("vad", True)
                text, dt_ms, info = transcribe_once(model, wav, language, vad)
                emit({
                    "status": "ok",
                    "text": text,
                    "duration_ms": dt_ms,
                    "language": info.language,
                    "language_probability": info.language_probability,
                })
                continue

            if cmd == "stream_start":
                streaming = True
                stream_lang = req.get("language", "en")
                if stream_lang == "auto":
                    stream_lang = None
                stream_vad = req.get("vad", True)
                last_partial_text = ""
                emit({"status": "streaming"})
                continue

            if cmd == "stream_chunk":
                if not streaming:
                    emit({"status": "error", "message": "stream_chunk before stream_start"})
                    continue
                wav = req["wav"]
                seq = int(req.get("seq", 0))
                try:
                    # Fast partial: beam=1, best_of=1 for low latency.
                    text, dt_ms, _ = transcribe_once(
                        model, wav, stream_lang, stream_vad,
                        beam_size=1, best_of=1,
                    )
                    last_partial_text = text
                    emit({"event": "partial", "seq": seq, "text": text, "duration_ms": dt_ms})
                finally:
                    try:
                        os.unlink(wav)
                    except OSError:
                        pass
                continue

            if cmd == "stream_stop":
                if not streaming:
                    emit({"status": "error", "message": "stream_stop before stream_start"})
                    continue
                streaming = False
                emit({"event": "final", "text": last_partial_text, "duration_ms": 0})
                emit({"status": "idle"})
                last_partial_text = ""
                continue

            emit({"status": "error", "message": f"unknown cmd {cmd}"})

        except Exception as e:
            emit({"status": "error", "message": str(e)})


if __name__ == "__main__":
    main()
