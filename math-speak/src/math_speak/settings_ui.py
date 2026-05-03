"""Tkinter settings window for math-speak.

Opened from the tray menu or via `math-speak --settings`. Reads/writes
`~/.config/math-speak/config.toml` and signals the running daemon to reload.
"""
from __future__ import annotations

import logging
import os
import signal
import subprocess
import threading
import tkinter as tk
from pathlib import Path
from tkinter import messagebox, ttk

from . import config as cfgmod

log = logging.getLogger("math_speak.settings")


def _list_voices(model_dir: Path) -> list[str]:
    voices: set[str] = set()
    if not model_dir.exists():
        return []
    for p in model_dir.glob("*.onnx"):
        voices.add(p.stem)
    return sorted(voices)


def _pid_of_daemon() -> int | None:
    try:
        out = subprocess.run(
            ["systemctl", "--user", "show", "-p", "MainPID", "--value", "math-speakd.service"],
            capture_output=True, text=True, check=False, timeout=2,
        )
        pid = int(out.stdout.strip() or "0")
        return pid if pid > 0 else None
    except Exception:
        return None


def _signal_reload() -> None:
    pid = _pid_of_daemon()
    if pid:
        try:
            os.kill(pid, signal.SIGHUP)
            return
        except ProcessLookupError:
            pass
    # fallback: socket trigger if running outside systemd
    sock = Path("/run/user") / str(os.getuid()) / "math-speak.sock"
    if not sock.exists():
        sock = cfgmod.STATE_DIR / "math-speak.sock"
    if sock.exists():
        try:
            import socket
            s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            s.connect(str(sock))
            s.sendall(b"reload")
            s.close()
        except OSError:
            pass


def _stop_daemon(disable: bool) -> tuple[bool, str]:
    cmds: list[list[str]] = []
    if disable:
        cmds.append(["systemctl", "--user", "disable", "--now", "math-speakd.service"])
    else:
        cmds.append(["systemctl", "--user", "stop", "math-speakd.service"])
    last_err = ""
    for cmd in cmds:
        r = subprocess.run(cmd, capture_output=True, text=True, check=False, timeout=10)
        if r.returncode != 0:
            last_err = (r.stderr or r.stdout).strip()
    # If systemctl call failed (e.g. running outside a user systemd session),
    # fall back to socket "quit".
    if last_err:
        sock = Path("/run/user") / str(os.getuid()) / "math-speak.sock"
        if not sock.exists():
            sock = cfgmod.STATE_DIR / "math-speak.sock"
        if sock.exists():
            try:
                import socket
                s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                s.connect(str(sock))
                s.sendall(b"quit")
                s.close()
                return True, ""
            except OSError as e:
                return False, f"{last_err}; socket fallback: {e}"
        return False, last_err
    return True, ""


class SettingsWindow:
    def __init__(self) -> None:
        self.cfg = cfgmod.load()
        self.root = tk.Tk()
        self.root.title("math-speak settings")
        self.root.geometry("520x540")
        try:
            self.root.tk.call("source", "azure.tcl")  # nice if installed; harmless if not
        except tk.TclError:
            pass

        style = ttk.Style(self.root)
        try:
            style.theme_use("clam")
        except tk.TclError:
            pass

        self._build()

    def _build(self) -> None:
        PX, PY = 10, 6
        frm = ttk.Frame(self.root, padding=12)
        frm.pack(fill="both", expand=True)

        # --- Hotkey ---
        ttk.Label(frm, text="Hotkey").grid(row=0, column=0, sticky="w", padx=PX, pady=PY)
        self.var_hotkey = tk.StringVar(value=self.cfg.hotkey)
        ttk.Entry(frm, textvariable=self.var_hotkey, width=28).grid(
            row=0, column=1, sticky="ew", padx=PX, pady=PY
        )
        ttk.Label(
            frm,
            text="(restart daemon after changing — pynput format, e.g. <ctrl>+<alt>+m)",
            foreground="#666",
        ).grid(row=1, column=0, columnspan=2, sticky="w", padx=PX)

        # --- Language ---
        ttk.Label(frm, text="Language").grid(row=2, column=0, sticky="w", padx=PX, pady=PY)
        self.var_lang = tk.StringVar(value=self.cfg.language)
        lang_frame = ttk.Frame(frm)
        lang_frame.grid(row=2, column=1, sticky="w", padx=PX, pady=PY)
        ttk.Radiobutton(lang_frame, text="English", value="en", variable=self.var_lang).pack(side="left", padx=6)
        ttk.Radiobutton(lang_frame, text="Français", value="fr", variable=self.var_lang).pack(side="left", padx=6)

        # --- Voices ---
        voices = _list_voices(self.cfg.expanded_model_dir())
        en_voices = [v for v in voices if v.startswith("en_")] or [self.cfg.piper_voice_en]
        fr_voices = [v for v in voices if v.startswith("fr_")] or [self.cfg.piper_voice_fr]

        ttk.Label(frm, text="EN voice").grid(row=3, column=0, sticky="w", padx=PX, pady=PY)
        self.var_voice_en = tk.StringVar(value=self.cfg.piper_voice_en)
        ttk.Combobox(
            frm, textvariable=self.var_voice_en, values=en_voices, state="readonly", width=30
        ).grid(row=3, column=1, sticky="ew", padx=PX, pady=PY)

        ttk.Label(frm, text="FR voice").grid(row=4, column=0, sticky="w", padx=PX, pady=PY)
        self.var_voice_fr = tk.StringVar(value=self.cfg.piper_voice_fr)
        ttk.Combobox(
            frm, textvariable=self.var_voice_fr, values=fr_voices, state="readonly", width=30
        ).grid(row=4, column=1, sticky="ew", padx=PX, pady=PY)

        # --- SRE phrasing domain ---
        ttk.Label(frm, text="Phrasing").grid(row=5, column=0, sticky="w", padx=PX, pady=PY)
        self.var_domain = tk.StringVar(value=self.cfg.sre_domain)
        dom_frame = ttk.Frame(frm)
        dom_frame.grid(row=5, column=1, sticky="w", padx=PX, pady=PY)
        ttk.Radiobutton(dom_frame, text="ClearSpeak (natural)", value="clearspeak", variable=self.var_domain).pack(side="left", padx=4)
        ttk.Radiobutton(dom_frame, text="MathSpeak (formal)", value="mathspeak", variable=self.var_domain).pack(side="left", padx=4)

        # --- Speed ---
        ttk.Label(frm, text="Speed").grid(row=6, column=0, sticky="w", padx=PX, pady=PY)
        self.var_speed = tk.DoubleVar(value=self.cfg.speed)
        speed_frame = ttk.Frame(frm)
        speed_frame.grid(row=6, column=1, sticky="ew", padx=PX, pady=PY)
        self.lbl_speed = ttk.Label(speed_frame, text=f"{self.cfg.speed:.2f}×", width=6)
        self.lbl_speed.pack(side="right")
        ttk.Scale(
            speed_frame,
            from_=0.5,
            to=2.0,
            variable=self.var_speed,
            orient="horizontal",
            command=lambda v: self.lbl_speed.config(text=f"{float(v):.2f}×"),
        ).pack(side="left", fill="x", expand=True)

        # --- Toggles ---
        self.var_raw = tk.BooleanVar(value=self.cfg.raw_mode)
        ttk.Checkbutton(
            frm,
            text="Raw mode (skip math normalizer; speak text via espeak-ng)",
            variable=self.var_raw,
        ).grid(row=7, column=0, columnspan=2, sticky="w", padx=PX, pady=PY)

        self.var_espeak = tk.BooleanVar(value=self.cfg.espeak_fallback)
        ttk.Checkbutton(
            frm, text="Fallback to espeak-ng if Piper / SRE fail", variable=self.var_espeak
        ).grid(row=8, column=0, columnspan=2, sticky="w", padx=PX, pady=PY)

        # --- Buttons ---
        btn_frame = ttk.Frame(frm)
        btn_frame.grid(row=9, column=0, columnspan=2, sticky="ew", pady=(14, 4))
        for i in range(3):
            btn_frame.columnconfigure(i, weight=1)

        ttk.Button(btn_frame, text="Save & apply", command=self._save).grid(row=0, column=0, sticky="ew", padx=4)
        ttk.Button(btn_frame, text="Test EN", command=lambda: self._test("en")).grid(row=0, column=1, sticky="ew", padx=4)
        ttk.Button(btn_frame, text="Test FR", command=lambda: self._test("fr")).grid(row=0, column=2, sticky="ew", padx=4)

        ttk.Separator(frm, orient="horizontal").grid(row=10, column=0, columnspan=2, sticky="ew", pady=10)

        # Daemon controls
        dctrl = ttk.LabelFrame(frm, text="Daemon", padding=8)
        dctrl.grid(row=11, column=0, columnspan=2, sticky="ew")
        for i in range(3):
            dctrl.columnconfigure(i, weight=1)

        self.lbl_status = ttk.Label(dctrl, text=self._status_text())
        self.lbl_status.grid(row=0, column=0, columnspan=3, sticky="w", pady=(0, 6))

        ttk.Button(dctrl, text="Refresh", command=self._refresh_status).grid(row=1, column=0, sticky="ew", padx=2)
        ttk.Button(dctrl, text="Stop now", command=lambda: self._quit_daemon(disable=False)).grid(row=1, column=1, sticky="ew", padx=2)
        ttk.Button(dctrl, text="Stop & disable autostart", command=lambda: self._quit_daemon(disable=True)).grid(row=1, column=2, sticky="ew", padx=2)

        frm.columnconfigure(1, weight=1)

    # --- Actions ---

    def _gather(self) -> cfgmod.Config:
        c = cfgmod.Config()
        c.hotkey = self.var_hotkey.get().strip() or c.hotkey
        c.language = self.var_lang.get()
        c.piper_voice_en = self.var_voice_en.get()
        c.piper_voice_fr = self.var_voice_fr.get()
        c.sre_domain = self.var_domain.get()
        c.speed = round(float(self.var_speed.get()), 2)
        c.raw_mode = bool(self.var_raw.get())
        c.espeak_fallback = bool(self.var_espeak.get())
        # preserve unchanged fields from disk
        c.llm_endpoint = self.cfg.llm_endpoint
        c.llm_model = self.cfg.llm_model
        c.llm_timeout_s = self.cfg.llm_timeout_s
        c.model_dir = self.cfg.model_dir
        return c

    def _save(self) -> None:
        new_cfg = self._gather()
        cfgmod.save(new_cfg)
        self.cfg = new_cfg
        _signal_reload()
        messagebox.showinfo(
            "math-speak",
            "Settings saved. Daemon reloaded.\n\n"
            "Note: changes to the hotkey only take effect after a daemon restart.",
        )

    def _test(self, lang: str) -> None:
        # Save first so the daemon uses current values; then synth in a thread.
        self._save_silent()
        threading.Thread(target=self._test_worker, args=(lang,), daemon=True).start()

    def _save_silent(self) -> None:
        new_cfg = self._gather()
        cfgmod.save(new_cfg)
        self.cfg = new_cfg
        _signal_reload()

    def _test_worker(self, lang: str) -> None:
        from .audio import play
        from .normalize import normalize as run_normalize
        from .tts import synthesize

        cfg = cfgmod.load()
        cfg.language = lang
        sample = "the integral from zero to one of x squared d x" if lang == "en" else (
            "la somme de un à n de x indice i au carré"
        )
        spoken, engine = run_normalize(sample, cfg)
        out = synthesize(spoken, engine, cfg)
        if out:
            play(*out)

    def _status_text(self) -> str:
        pid = _pid_of_daemon()
        if pid:
            return f"Daemon running (pid {pid})"
        sock = Path("/run/user") / str(os.getuid()) / "math-speak.sock"
        if not sock.exists():
            sock = cfgmod.STATE_DIR / "math-speak.sock"
        if sock.exists():
            return "Daemon running (foreground)"
        return "Daemon stopped"

    def _refresh_status(self) -> None:
        self.lbl_status.config(text=self._status_text())

    def _quit_daemon(self, disable: bool) -> None:
        msg = (
            "Stop the math-speak daemon now and disable autostart?\n\n"
            "Re-enable later with:\n  systemctl --user enable --now math-speakd"
            if disable
            else "Stop the math-speak daemon now? It will restart at next login."
        )
        if not messagebox.askyesno("math-speak", msg):
            return
        ok, err = _stop_daemon(disable=disable)
        if not ok:
            messagebox.showerror("math-speak", f"Could not stop daemon:\n{err}")
        else:
            self._refresh_status()
            messagebox.showinfo(
                "math-speak",
                "Daemon stopped." + (" Autostart disabled." if disable else ""),
            )
            self.root.after(500, self.root.destroy)

    def run(self) -> None:
        self.root.mainloop()


def open_window() -> None:
    """Open the settings window. Blocks until closed."""
    SettingsWindow().run()
