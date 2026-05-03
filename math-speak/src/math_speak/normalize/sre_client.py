from __future__ import annotations

import json
import logging
import os
import subprocess
import threading
import time
from pathlib import Path

log = logging.getLogger("math_speak.sre")

_lock = threading.Lock()
_proc: subprocess.Popen | None = None
_req_id = 0


def _resolve_daemon() -> Path | None:
    if env := os.environ.get("MATH_SPEAK_SRE_DAEMON"):
        return Path(env)
    candidates = [
        Path.home() / ".local/share/math-speak/node/sre_daemon.js",
        Path(__file__).resolve().parents[3] / "node" / "sre_daemon.js",  # dev tree
        Path(__file__).resolve().parents[4] / "node" / "sre_daemon.js",  # editable install
    ]
    for c in candidates:
        if c.exists() and (c.parent / "node_modules").exists():
            return c
    return None


def _start() -> subprocess.Popen | None:
    daemon = _resolve_daemon()
    if daemon is None:
        log.warning("SRE daemon not found (set MATH_SPEAK_SRE_DAEMON to override)")
        return None
    daemon_path = str(daemon)
    try:
        proc = subprocess.Popen(
            ["node", daemon_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            bufsize=1,
            text=True,
        )
    except FileNotFoundError:
        log.warning("node not on PATH; SRE disabled")
        return None
    # wait for ready line
    deadline = time.time() + 5.0
    while time.time() < deadline:
        line = proc.stdout.readline() if proc.stdout else ""
        if not line:
            if proc.poll() is not None:
                return None
            continue
        try:
            msg = json.loads(line)
            if msg.get("status") == "ready":
                log.info("SRE daemon ready")
                return proc
        except json.JSONDecodeError:
            continue
    proc.kill()
    return None


def _ensure() -> subprocess.Popen | None:
    global _proc
    if _proc is None or _proc.poll() is not None:
        _proc = _start()
    return _proc


def to_speech(text: str, kind: str, lang: str, domain: str | None = None) -> str:
    """kind: latex|mathml|asciimath. lang: en|fr. domain: clearspeak|mathspeak."""
    global _req_id
    with _lock:
        proc = _ensure()
        if proc is None or proc.stdin is None or proc.stdout is None:
            return ""
        _req_id += 1
        rid = _req_id
        req = {"id": rid, "input": text, "type": kind, "locale": lang}
        if domain:
            req["domain"] = domain
        try:
            proc.stdin.write(json.dumps(req) + "\n")
            proc.stdin.flush()
        except (BrokenPipeError, OSError):
            return ""
        deadline = time.time() + 4.0
        while time.time() < deadline:
            line = proc.stdout.readline()
            if not line:
                if proc.poll() is not None:
                    return ""
                continue
            try:
                resp = json.loads(line)
            except json.JSONDecodeError:
                continue
            if resp.get("id") == rid:
                return (resp.get("speech") or "").strip()
        log.warning("SRE timeout for kind=%s lang=%s", kind, lang)
        return ""


def shutdown() -> None:
    global _proc
    if _proc and _proc.poll() is None:
        try:
            _proc.terminate()
            _proc.wait(timeout=1.0)
        except Exception:
            _proc.kill()
    _proc = None
