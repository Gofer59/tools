#!/usr/bin/env python3
"""Deck Reader — GUI status window with Quit button.

Spawned by the deck-reader Rust binary. Reads status lines from stdin
to update the display. Closing the window or clicking Quit sends SIGTERM
to the parent process (the Rust binary).

Usage (called by Rust, not manually):
    python3 gui_window.py
"""
import os
import signal
import sys
import threading
import tkinter as tk


def kill_parent():
    """Send SIGTERM to the parent Rust process."""
    try:
        os.kill(os.getppid(), signal.SIGTERM)
    except OSError:
        pass


def read_stdin(root, status_var, ocr_var, tts_var):
    """Background thread: read status lines from stdin and update labels."""
    try:
        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue

            # Protocol: "key:value" lines
            if line.startswith("status:"):
                val = line[len("status:"):]
                root.after(0, status_var.set, val)
            elif line.startswith("ocr:"):
                val = line[len("ocr:"):]
                root.after(0, ocr_var.set, val)
            elif line.startswith("tts:"):
                val = line[len("tts:"):]
                root.after(0, tts_var.set, val)
    except (EOFError, BrokenPipeError, ValueError):
        pass

    # stdin closed (parent exited) — close the window
    try:
        root.after(0, root.destroy)
    except Exception:
        pass


def main():
    root = tk.Tk()
    root.title("Deck Reader")
    root.resizable(False, False)

    # ── Status variables ──────────────────────────────────────────────────
    status_var = tk.StringVar(value="Listening...")
    ocr_var = tk.StringVar(value="—")
    tts_var = tk.StringVar(value="idle")

    # ── Layout ────────────────────────────────────────────────────────────
    frame = tk.Frame(root, padx=16, pady=12)
    frame.pack()

    title = tk.Label(frame, text="Deck Reader", font=("sans-serif", 14, "bold"))
    title.pack(pady=(0, 8))

    sep1 = tk.Frame(frame, height=1, bg="gray70")
    sep1.pack(fill="x", pady=4)

    info_frame = tk.Frame(frame)
    info_frame.pack(fill="x", pady=4)

    for row, (label_text, var) in enumerate([
        ("Status:", status_var),
        ("Last OCR:", ocr_var),
        ("TTS:", tts_var),
    ]):
        lbl = tk.Label(info_frame, text=label_text, anchor="w", width=10)
        lbl.grid(row=row, column=0, sticky="w", pady=2)
        val = tk.Label(info_frame, textvariable=var, anchor="w", width=30)
        val.grid(row=row, column=1, sticky="w", pady=2)

    sep2 = tk.Frame(frame, height=1, bg="gray70")
    sep2.pack(fill="x", pady=8)

    quit_btn = tk.Button(
        frame, text="Quit", width=10, command=lambda: on_quit(root)
    )
    quit_btn.pack()

    # ── Close window = quit ───────────────────────────────────────────────
    def on_quit(r):
        kill_parent()
        r.destroy()

    root.protocol("WM_DELETE_WINDOW", lambda: on_quit(root))

    # ── Background stdin reader thread ────────────────────────────────────
    reader = threading.Thread(
        target=read_stdin, args=(root, status_var, ocr_var, tts_var),
        daemon=True
    )
    reader.start()

    root.mainloop()


if __name__ == "__main__":
    main()
