from __future__ import annotations

import os
import subprocess
import time

_last_primary: str = ""


def _is_wayland() -> bool:
    return bool(os.environ.get("WAYLAND_DISPLAY")) or os.environ.get(
        "XDG_SESSION_TYPE", ""
    ).lower() == "wayland"


def _read(selection: str, timeout_s: float = 0.2) -> str:
    if _is_wayland():
        cmd = ["wl-paste", "--no-newline"]
        if selection == "primary":
            cmd.insert(1, "--primary")
    else:
        cmd = ["xclip", "-selection", selection, "-o"]
    try:
        out = subprocess.run(cmd, capture_output=True, timeout=timeout_s, check=False)
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return ""
    if out.returncode != 0:
        return ""
    return out.stdout.decode("utf-8", errors="replace").strip()


def get_selection(debounce_s: float = 0.1) -> str:
    """Read PRIMARY then CLIPBOARD if PRIMARY is unchanged or empty.

    Mirrors voice-speak/src-tauri/src/lib.rs:215-228 stale-PRIMARY guard.
    """
    global _last_primary
    if debounce_s > 0:
        time.sleep(debounce_s)
    primary = _read("primary")
    if primary and primary != _last_primary:
        _last_primary = primary
        return primary
    clip = _read("clipboard")
    if clip:
        return clip
    if primary:
        _last_primary = primary
        return primary
    return ""
