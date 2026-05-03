from __future__ import annotations

import argparse
import logging
import socket
import sys
from pathlib import Path

from . import config as cfgmod
from . import selection, tts
from .audio import play
from .daemon import SOCKET_PATH
from .normalize import normalize as run_normalize


def _send(cmd: str) -> str:
    if not Path(SOCKET_PATH).exists():
        return ""
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    try:
        s.connect(str(SOCKET_PATH))
        s.sendall(cmd.encode("utf-8"))
        return s.recv(64).decode("utf-8", errors="ignore").strip()
    except OSError:
        return ""
    finally:
        s.close()


def _selftest() -> int:
    cases = [
        ("∫₀¹ x² dx", "en"),
        ("∫₀¹ x² dx", "fr"),
        (r"\frac{\partial L}{\partial \theta}", "en"),
        (r"\frac{\partial L}{\partial \theta}", "fr"),
        (r"sum_{i=1}^n sqrt(x_i)", "en"),
        (r"sum_{i=1}^n sqrt(x_i)", "fr"),
        ("f : ℝⁿ → ℝ, f(x) = ‖x‖²", "en"),
        ("f : ℝⁿ → ℝ, f(x) = ‖x‖²", "fr"),
    ]
    base = cfgmod.load()
    fails = 0
    for i, (text, lang) in enumerate(cases, 1):
        cfg = cfgmod.Config(**{**base.__dict__, "language": lang})
        spoken, engine = run_normalize(text, cfg)
        ok = bool(spoken.strip())
        print(f"[{i}/8] lang={lang:<2} engine={engine:<6} ok={ok} :: {text!r}")
        print(f"        → {spoken!r}")
        if not ok:
            fails += 1
    print(f"\n{8 - fails}/8 passed")
    return 1 if fails else 0


def _say(text: str, lang: str | None) -> int:
    cfg = cfgmod.load()
    if lang:
        cfg.language = lang
    spoken, engine = run_normalize(text, cfg)
    print(f"normalized ({engine}): {spoken}", file=sys.stderr)
    out = tts.synthesize(spoken, engine, cfg)
    if out is None:
        print("no audio", file=sys.stderr)
        return 2
    pcm, sr = out
    play(pcm, sr)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(prog="math-speak", description="math-speak CLI trigger")
    parser.add_argument("--trigger", action="store_true", help="ask running daemon to read selection")
    parser.add_argument("--lang", choices=["en", "fr"], help="set language (persists to config)")
    parser.add_argument("--say", metavar="TEXT", help="synthesize TEXT directly (bypass daemon)")
    parser.add_argument("--selftest", action="store_true", help="run 8-case smoke matrix (text only, no audio)")
    parser.add_argument("--read-selection", action="store_true", help="read current PRIMARY/CLIPBOARD selection now")
    parser.add_argument("--settings", action="store_true", help="open the settings window")
    parser.add_argument("--quit", action="store_true", help="stop the running daemon (systemctl --user stop)")
    parser.add_argument("--disable", action="store_true", help="stop daemon AND disable autostart (use with --quit)")
    parser.add_argument("-v", "--verbose", action="count", default=0)
    args = parser.parse_args()

    if args.verbose:
        logging.basicConfig(level=logging.DEBUG, format="%(name)s %(levelname)s %(message)s")
    else:
        logging.basicConfig(level=logging.INFO, format="%(message)s")

    if args.selftest:
        return _selftest()

    if args.settings:
        from . import settings_ui
        settings_ui.open_window()
        return 0

    if args.quit:
        from .settings_ui import _stop_daemon
        ok, err = _stop_daemon(disable=args.disable)
        if not ok:
            print(f"failed: {err}", file=sys.stderr)
            return 1
        print("daemon stopped" + (" and autostart disabled" if args.disable else ""))
        return 0

    if args.lang:
        # Try to update via daemon; fall back to direct write
        if not _send(f"lang {args.lang}"):
            cfgmod.set_language(args.lang)
        print(f"language set to {args.lang}")
        if not args.trigger and not args.say and not args.read_selection:
            return 0

    if args.say:
        return _say(args.say, args.lang)

    if args.read_selection:
        text = selection.get_selection(debounce_s=0)
        return _say(text, args.lang) if text else 1

    if args.trigger or len(sys.argv) == 1:
        resp = _send("trigger")
        if not resp:
            print("daemon not running; start with: math-speakd --foreground", file=sys.stderr)
            return 1
        return 0

    parser.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
