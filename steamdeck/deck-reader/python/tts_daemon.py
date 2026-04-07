#!/usr/bin/env python3
"""
tts_daemon.py — Persistent TTS daemon with streaming audio playback.

Keeps the Piper ONNX model loaded in memory and streams synthesized audio
chunk-by-chunk to ``paplay --raw`` via stdin.  Communicates with the Rust
binary over a Unix domain socket using newline-delimited JSON.

Started by the Rust binary on first TTS request:

    python3 tts_daemon.py <socket_path> <models_dir>

Protocol (newline-delimited JSON):

    Rust -> Daemon:
        {"text": "...", "voice": "...", "speed": 1.0}   — speak request
        {"cmd": "shutdown"}                              — graceful exit

    Daemon -> Rust:
        {"status": "playing", "pgid": 12345}             — audio started
        {"status": "done"}                                — playback finished
        {"status": "error", "msg": "..."}                 — failure
"""

from __future__ import annotations

import atexit
import glob
import json
import os
import signal
import socket
import subprocess
import sys


# ─────────────────────────────────────────────────────────────────────────────
# Globals
# ─────────────────────────────────────────────────────────────────────────────

_voice_name: str | None = None
_voice_obj = None          # piper.PiperVoice
_sample_rate: int = 22050  # updated when a model is loaded
_current_paplay: subprocess.Popen | None = None
_sock: socket.socket | None = None
_socket_path: str | None = None


# ─────────────────────────────────────────────────────────────────────────────
# Startup
# ─────────────────────────────────────────────────────────────────────────────

def parse_args() -> tuple[str, str]:
    if len(sys.argv) < 3:
        print("Usage: tts_daemon.py <socket_path> <models_dir>", file=sys.stderr)
        sys.exit(1)
    return sys.argv[1], sys.argv[2]


def import_piper():
    """One-time import of piper and its config module."""
    global piper, SynthesisConfig
    try:
        import piper as _piper
        from piper.config import SynthesisConfig as _SynthesisConfig
        piper = _piper
        SynthesisConfig = _SynthesisConfig
    except ImportError:
        print(
            "ERROR: piper-tts is not installed.\n"
            "Run:  pip install piper-tts",
            file=sys.stderr,
        )
        sys.exit(1)


def setup_socket(socket_path: str) -> socket.socket:
    global _sock, _socket_path
    _socket_path = socket_path

    # Remove stale socket from a previous crash.
    try:
        os.unlink(socket_path)
    except FileNotFoundError:
        pass

    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.bind(socket_path)
    sock.listen(1)
    _sock = sock

    atexit.register(_cleanup_socket)
    return sock


def _cleanup_socket():
    if _socket_path:
        try:
            os.unlink(_socket_path)
        except FileNotFoundError:
            pass


def _handle_sigterm(_signum, _frame):
    _kill_paplay()
    _cleanup_socket()
    sys.exit(0)


# ─────────────────────────────────────────────────────────────────────────────
# Model management
# ─────────────────────────────────────────────────────────────────────────────

def _find_model(models_dir: str, voice: str) -> str | None:
    """Locate the .onnx file for the given voice name (same logic as tts_speak.py)."""
    # Nested layout: models/<voice>*/<voice>*.onnx
    pattern = os.path.join(models_dir, f"{voice}*", f"{voice}*.onnx")
    files = glob.glob(pattern)
    if files:
        return files[0]

    # Flat layout: models/<voice>.onnx
    pattern = os.path.join(models_dir, f"{voice}.onnx")
    files = glob.glob(pattern)
    if files:
        return files[0]

    return None


def _ensure_model(voice: str, models_dir: str) -> bool:
    """Load or reload the Piper model if the voice name changed.  Returns True on success."""
    global _voice_name, _voice_obj, _sample_rate

    if _voice_name == voice and _voice_obj is not None:
        return True

    model_path = _find_model(models_dir, voice)
    if model_path is None:
        print(
            f"[tts-daemon] ERROR: No model found for '{voice}' in {models_dir}",
            file=sys.stderr,
        )
        return False

    print(f"[tts-daemon] Loading model: {model_path}", file=sys.stderr)
    _voice_obj = piper.PiperVoice.load(model_path)
    _voice_name = voice
    _sample_rate = _voice_obj.config.sample_rate
    print(
        f"[tts-daemon] Model loaded (sample_rate={_sample_rate})",
        file=sys.stderr,
    )
    return True


# ─────────────────────────────────────────────────────────────────────────────
# Playback management
# ─────────────────────────────────────────────────────────────────────────────

def _kill_paplay():
    """Kill the current paplay process group if it is still running."""
    global _current_paplay
    if _current_paplay is not None:
        try:
            _current_paplay.poll()
            if _current_paplay.returncode is None:
                os.killpg(_current_paplay.pid, signal.SIGKILL)
                _current_paplay.wait()
        except (ProcessLookupError, OSError):
            pass
        _current_paplay = None


def _spawn_paplay() -> subprocess.Popen:
    """Start a paplay process that reads raw s16le PCM from stdin."""
    proc = subprocess.Popen(
        [
            "paplay",
            f"--format=s16le",
            f"--rate={_sample_rate}",
            "--channels=1",
            "--raw",
        ],
        stdin=subprocess.PIPE,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        preexec_fn=os.setsid,
    )
    return proc


# ─────────────────────────────────────────────────────────────────────────────
# Request handling
# ─────────────────────────────────────────────────────────────────────────────

def _send(writer, obj: dict):
    """Write a JSON line to the Rust side."""
    try:
        writer.write(json.dumps(obj) + "\n")
        writer.flush()
    except BrokenPipeError:
        pass


def handle_speak(request: dict, writer, models_dir: str):
    global _current_paplay
    import time

    t0 = time.monotonic()

    text = request.get("text", "").strip()
    voice = request.get("voice", "en_US-lessac-medium")
    speed = float(request.get("speed", 1.0))

    if not text:
        _send(writer, {"status": "done"})
        return

    # Load / reload model if voice changed.
    if not _ensure_model(voice, models_dir):
        _send(writer, {"status": "error", "msg": f"No model for '{voice}'"})
        return

    t1 = time.monotonic()
    print(f"[tts-daemon] TIMING model_check={1000*(t1-t0):.0f}ms", file=sys.stderr)

    # Kill previous paplay if still running (preemption).
    _kill_paplay()

    # Spawn paplay for raw PCM streaming.
    try:
        proc = _spawn_paplay()
    except FileNotFoundError:
        _send(writer, {"status": "error", "msg": "paplay not found"})
        return
    except OSError as e:
        _send(writer, {"status": "error", "msg": f"paplay spawn failed: {e}"})
        return

    _current_paplay = proc

    t2 = time.monotonic()
    print(f"[tts-daemon] TIMING paplay_spawn={1000*(t2-t1):.0f}ms", file=sys.stderr)

    # Tell Rust the paplay PGID so it can kill audio instantly.
    _send(writer, {"status": "playing", "pgid": proc.pid})

    # Stream synthesis chunks to paplay stdin.
    syn_config = SynthesisConfig(length_scale=1.0 / speed)
    chunk_count = 0
    try:
        for chunk in _voice_obj.synthesize(text, syn_config=syn_config):
            if chunk_count == 0:
                t3 = time.monotonic()
                print(
                    f"[tts-daemon] TIMING first_chunk={1000*(t3-t0):.0f}ms "
                    f"(synth={1000*(t3-t2):.0f}ms, "
                    f"bytes={len(chunk.audio_int16_bytes)})",
                    file=sys.stderr,
                )
            proc.stdin.write(chunk.audio_int16_bytes)
            proc.stdin.flush()
            chunk_count += 1
    except BrokenPipeError:
        # paplay was killed (user pressed stop or Rust killed it).
        print("[tts-daemon] Playback interrupted (BrokenPipe).", file=sys.stderr)
        _current_paplay = None
        return
    except Exception as e:
        print(f"[tts-daemon] Synthesis error: {e}", file=sys.stderr)
        _kill_paplay()
        _send(writer, {"status": "error", "msg": str(e)})
        return

    t4 = time.monotonic()
    print(
        f"[tts-daemon] TIMING total={1000*(t4-t0):.0f}ms "
        f"(chunks={chunk_count})",
        file=sys.stderr,
    )

    # Close stdin to signal EOF, then wait for paplay to finish draining.
    try:
        proc.stdin.close()
    except BrokenPipeError:
        pass
    proc.wait()
    _current_paplay = None

    _send(writer, {"status": "done"})


def handle_connection(conn: socket.socket, models_dir: str):
    """Handle one Rust connection.  Returns when the connection closes."""
    reader = conn.makefile("r", encoding="utf-8")
    writer = conn.makefile("w", encoding="utf-8")

    try:
        for line in reader:
            line = line.strip()
            if not line:
                continue
            try:
                request = json.loads(line)
            except json.JSONDecodeError as e:
                print(f"[tts-daemon] Bad JSON: {e}", file=sys.stderr)
                _send(writer, {"status": "error", "msg": f"bad JSON: {e}"})
                continue

            if request.get("cmd") == "shutdown":
                print("[tts-daemon] Shutdown requested.", file=sys.stderr)
                _kill_paplay()
                _cleanup_socket()
                sys.exit(0)

            handle_speak(request, writer, models_dir)
    except (ConnectionResetError, BrokenPipeError):
        print("[tts-daemon] Client disconnected.", file=sys.stderr)
    finally:
        _kill_paplay()
        reader.close()
        writer.close()
        conn.close()


# ─────────────────────────────────────────────────────────────────────────────
# Main loop
# ─────────────────────────────────────────────────────────────────────────────

def main():
    socket_path, models_dir = parse_args()

    signal.signal(signal.SIGTERM, _handle_sigterm)
    # Ensure BrokenPipeError is raised (not silent SIGPIPE death) when writing
    # to a killed paplay's stdin.
    signal.signal(signal.SIGPIPE, signal.SIG_DFL)

    import_piper()

    sock = setup_socket(socket_path)

    # Signal to the Rust parent that we are ready to accept connections.
    print("READY", flush=True)

    print(f"[tts-daemon] Listening on {socket_path}", file=sys.stderr)

    while True:
        try:
            conn, _ = sock.accept()
        except OSError:
            break
        print("[tts-daemon] Client connected.", file=sys.stderr)
        handle_connection(conn, models_dir)
        print("[tts-daemon] Waiting for next client…", file=sys.stderr)


if __name__ == "__main__":
    main()
