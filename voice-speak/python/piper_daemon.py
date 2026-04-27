#!/usr/bin/env python3
"""Persistent Piper TTS daemon: stdin/stdout IPC.
Protocol:
  - Each response starts with a JSON line on stdout (text mode via sys.stdout.buffer + line encoding)
  - For "speaking" status: immediately followed by raw 16-bit signed LE PCM bytes
  - Audio chunks are prefixed with: {"audio_pcm_bytes": N, "sample_rate": 22050, "channels": 1, "id": "..."}\n
    followed by exactly N bytes of raw PCM.
"""
import json
import sys
import os
import glob

STDOUT = sys.stdout.buffer
STDIN = sys.stdin


def emit_json(d: dict) -> None:
    STDOUT.write((json.dumps(d) + "\n").encode("utf-8"))
    STDOUT.flush()


def emit_pcm_chunk(chunk_id: str, sample_rate: int, pcm_bytes: bytes) -> None:
    hdr = {
        "audio_pcm_bytes": len(pcm_bytes),
        "sample_rate": sample_rate,
        "channels": 1,
        "id": chunk_id,
    }
    STDOUT.write((json.dumps(hdr) + "\n").encode("utf-8"))
    STDOUT.write(pcm_bytes)
    STDOUT.flush()


def find_onnx(model_dir: str, voice: str) -> str | None:
    patterns = [
        os.path.join(model_dir, f"{voice}.onnx"),
        os.path.join(model_dir, f"**/{voice}.onnx"),
        os.path.join(model_dir, f"{voice}*/{voice}*.onnx"),
        os.path.join(model_dir, f"**/{voice}*.onnx"),
    ]
    for pat in patterns:
        for m in glob.glob(pat, recursive=True):
            return m
    return None


def main() -> None:
    if len(sys.argv) < 2:
        emit_json({"status": "error", "message": "usage: piper_daemon.py model_dir"})
        sys.exit(1)

    model_dir = sys.argv[1]
    current_voice: str | None = None
    voice_obj = None

    emit_json({"status": "ready"})

    for line in STDIN:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
            cmd = req.get("cmd", "")

            if cmd == "quit":
                emit_json({"status": "quitting"})
                return

            if cmd == "stop":
                emit_json({"status": "stopped", "id": req.get("id", "")})
                continue

            if cmd != "speak":
                emit_json({"status": "error", "message": f"unknown cmd: {cmd}"})
                continue

            voice = req["voice"]
            text = req["text"]
            speed = float(req.get("speed", 1.0))
            noise_scale = float(req.get("noise_scale", 0.667))
            noise_w_scale = float(req.get("noise_w_scale", 0.8))
            chunk_id = req.get("id", "")

            if voice != current_voice:
                onnx_path = find_onnx(model_dir, voice)
                if onnx_path is None:
                    emit_json({"status": "error", "id": chunk_id, "message": f"voice not found: {voice}"})
                    continue
                try:
                    from piper import PiperVoice
                    voice_obj = PiperVoice.load(onnx_path)
                    current_voice = voice
                except Exception as e:
                    emit_json({"status": "error", "id": chunk_id, "message": f"load failed: {e}"})
                    continue

            emit_json({"status": "speaking", "id": chunk_id})

            try:
                import numpy as np
                from piper.config import SynthesisConfig
                syn = SynthesisConfig(
                    length_scale=1.0 / max(speed, 0.1),
                    noise_scale=noise_scale,
                    noise_w_scale=noise_w_scale,
                )
                for chunk in voice_obj.synthesize(text, syn_config=syn):
                    audio = chunk.audio_float_array
                    pcm = (np.clip(audio, -1.0, 1.0) * 32767).astype("<i2").tobytes()
                    emit_pcm_chunk(chunk_id, chunk.sample_rate, pcm)
            except AttributeError:
                # Older piper API fallback: synthesize returns raw bytes
                import io
                import wave
                buf = io.BytesIO()
                voice_obj.synthesize(text, buf)
                buf.seek(0)
                with wave.open(buf) as wf:
                    sample_rate = wf.getframerate()
                    raw = wf.readframes(wf.getnframes())
                emit_pcm_chunk(chunk_id, sample_rate, raw)

            emit_json({"status": "done", "id": chunk_id})

        except Exception as e:
            emit_json({"status": "error", "message": str(e)})


if __name__ == "__main__":
    main()
