from __future__ import annotations

import argparse
import logging
import os
import signal
import socket
import threading
import time
from pathlib import Path

from . import config as cfgmod
from . import hotkey, selection, tray, tts
from .audio import play
from .normalize import llm_rewrite
from .normalize import normalize as run_normalize

log = logging.getLogger("math_speak.daemon")

SOCKET_PATH = Path("/run/user") / str(os.getuid()) / "math-speak.sock"
if not SOCKET_PATH.parent.exists():
    SOCKET_PATH = cfgmod.STATE_DIR / "math-speak.sock"

_state = {
    "cfg": cfgmod.load(),
    "speaking": False,
}
_lock = threading.Lock()


def _reload_config(*_a) -> None:
    with _lock:
        _state["cfg"] = cfgmod.load()
    log.info("config reloaded; language=%s", _state["cfg"].language)


def _trigger_pipeline() -> None:
    if _state["speaking"]:
        log.info("trigger ignored; already speaking")
        return
    _state["speaking"] = True
    try:
        text = selection.get_selection()
        if not text:
            log.info("empty selection")
            return
        log.info("selection len=%d", len(text))
        cfg = _state["cfg"]
        spoken, engine = run_normalize(text, cfg)
        if not spoken:
            log.info("normalizer empty; skipping")
            return
        log.info("normalized via %s len=%d", engine, len(spoken))
        out = tts.synthesize(spoken, engine, cfg)
        if out is None:
            log.warning("TTS produced no audio")
            return
        pcm, sr = out
        play(pcm, sr)
    finally:
        _state["speaking"] = False


def _trigger_async() -> None:
    threading.Thread(target=_trigger_pipeline, daemon=True, name="math-speak-pipe").start()


def _open_settings() -> None:
    try:
        from . import settings_ui
        settings_ui.open_window()
    except Exception as e:
        log.warning("settings UI failed: %s", e)


def _serve_socket(stop_event: threading.Event) -> None:
    """Tiny UNIX-socket trigger so a CLI invocation (math-speak) can fire the daemon."""
    cfgmod.STATE_DIR.mkdir(parents=True, exist_ok=True)
    if SOCKET_PATH.exists():
        try:
            SOCKET_PATH.unlink()
        except OSError:
            pass
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.bind(str(SOCKET_PATH))
    s.listen(8)
    s.settimeout(0.5)
    log.info("trigger socket: %s", SOCKET_PATH)
    while not stop_event.is_set():
        try:
            conn, _ = s.accept()
        except TimeoutError:
            continue
        except OSError:
            break
        try:
            data = conn.recv(64).decode("utf-8", errors="ignore").strip()
            if data == "trigger" or data == "":
                _trigger_async()
                conn.sendall(b"ok\n")
            elif data.startswith("lang "):
                lang = data.split(" ", 1)[1].strip()
                cfgmod.set_language(lang)
                _reload_config()
                conn.sendall(b"ok\n")
            elif data == "reload":
                _reload_config()
                conn.sendall(b"ok\n")
            elif data == "quit":
                conn.sendall(b"ok\n")
                stop_event.set()
                break
            elif data == "settings":
                threading.Thread(target=_open_settings, daemon=True).start()
                conn.sendall(b"ok\n")
            else:
                conn.sendall(b"unknown\n")
        finally:
            conn.close()
    try:
        SOCKET_PATH.unlink()
    except OSError:
        pass


def main() -> int:
    parser = argparse.ArgumentParser(prog="math-speakd", description="math-speak background daemon")
    parser.add_argument("--foreground", action="store_true", help="(default) run in foreground")
    parser.add_argument("--no-tray", action="store_true", help="skip system tray icon")
    parser.add_argument("--no-hotkey", action="store_true", help="skip global hotkey registration")
    parser.add_argument("-v", "--verbose", action="count", default=0)
    args = parser.parse_args()

    level = logging.DEBUG if args.verbose else logging.INFO
    logging.basicConfig(
        level=level,
        format="%(asctime)s %(name)s %(levelname)s %(message)s",
    )

    cfg = _state["cfg"]
    stop_event = threading.Event()

    signal.signal(signal.SIGHUP, lambda *_: _reload_config())
    signal.signal(signal.SIGTERM, lambda *_: stop_event.set())
    signal.signal(signal.SIGINT, lambda *_: stop_event.set())

    # Warm LLM in background
    threading.Thread(target=llm_rewrite.warmup, args=(cfg,), daemon=True).start()

    if not args.no_hotkey:
        hotkey.start(cfg.hotkey, _trigger_async)

    if not args.no_tray:
        def get_lang() -> str:
            return _state["cfg"].language

        def set_lang(lang: str) -> None:
            cfgmod.set_language(lang)
            _reload_config()

        def quit_cb() -> None:
            stop_event.set()

        tray.start(get_lang, set_lang, quit_cb, _open_settings)

    sock_thread = threading.Thread(target=_serve_socket, args=(stop_event,), daemon=True)
    sock_thread.start()

    log.info("math-speakd ready; hotkey=%s lang=%s", cfg.hotkey, cfg.language)
    while not stop_event.is_set():
        time.sleep(0.5)
    log.info("math-speakd shutting down")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
