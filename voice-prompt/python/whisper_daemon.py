#!/usr/bin/env python3
"""Persistent Whisper daemon: stdin/stdout JSON-line IPC."""
import json
import sys
import time

from faster_whisper import WhisperModel


def log(msg):
    print(msg, file=sys.stderr, flush=True)


def main():
    if len(sys.argv) < 4:
        print(json.dumps({"status": "error", "message": "usage: model compute_type model_dir"}), flush=True)
        sys.exit(1)

    model_name, compute_type, model_dir = sys.argv[1], sys.argv[2], sys.argv[3]
    log(f"[whisperd] loading {model_name} compute={compute_type} dir={model_dir}")

    model = WhisperModel(
        model_size_or_path=model_name,
        device="cpu",
        compute_type=compute_type,
        download_root=model_dir,
    )

    print(json.dumps({"status": "ready", "model": model_name}), flush=True)

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
            cmd = req.get("cmd")

            if cmd == "quit":
                print(json.dumps({"status": "quitting"}), flush=True)
                return

            if cmd != "transcribe":
                print(json.dumps({"status": "error", "message": f"unknown cmd {cmd}"}), flush=True)
                continue

            t0 = time.time()
            wav = req["wav"]
            language = req.get("language", "en")
            if language == "auto":
                language = None
            vad = req.get("vad", True)

            segments, info = model.transcribe(
                wav,
                vad_filter=vad,
                language=language,
                beam_size=5,
                best_of=5,
                temperature=0.0,
            )

            text = " ".join(s.text.strip() for s in segments if s.text.strip())
            dt_ms = int((time.time() - t0) * 1000)

            print(json.dumps({
                "status": "ok",
                "text": text,
                "duration_ms": dt_ms,
                "language": info.language,
                "language_probability": info.language_probability,
            }), flush=True)

        except Exception as e:
            print(json.dumps({"status": "error", "message": str(e)}), flush=True)


if __name__ == "__main__":
    main()
