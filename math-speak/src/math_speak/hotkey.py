from __future__ import annotations

import logging
import threading
from collections.abc import Callable

log = logging.getLogger("math_speak.hotkey")


def start(combo: str, callback: Callable[[], None]) -> threading.Thread | None:
    """Bind a global hotkey and call `callback` on press. Returns the listener thread.

    Tries pynput.keyboard.GlobalHotKeys first; on failure (Wayland-restrictive
    compositor), nothing else is attempted in this minimal implementation —
    callers can fall back to the CLI trigger (math-speak --trigger) bound to a
    desktop-environment shortcut.
    """
    try:
        from pynput.keyboard import GlobalHotKeys
    except Exception as e:
        log.warning("pynput unavailable: %s", e)
        return None

    def _wrap() -> None:
        try:
            callback()
        except Exception as e:
            log.exception("hotkey callback raised: %s", e)

    try:
        listener = GlobalHotKeys({combo: _wrap})
        listener.daemon = True
        listener.start()
        log.info("hotkey listener started for %s", combo)
        return listener
    except Exception as e:
        log.warning("pynput GlobalHotKeys failed: %s", e)
        return None
