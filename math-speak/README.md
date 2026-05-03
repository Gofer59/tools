# math-speak

Selected-text TTS for mathematical content. Sibling of `voice-speak`.

Captures the current X11/Wayland text selection on **Ctrl+Alt+M**, classifies its
math format (Unicode glyphs / LaTeX / ASCII math / mixed PDF), normalizes it to
fluent spoken English or French, and synthesizes audio with **Piper** (with
**espeak-ng** as a fallback).

## Pipeline

```
selection → classify → normalize → TTS → audio
                         │
                         ├── pylatexenc (LaTeX text-mode pre-pass)
                         ├── Speech Rule Engine (Node, long-lived) — primary
                         ├── Unicode glyph table (regex)
                         └── local LLM via Ollama (gemma3n:e2b) — last resort
```

## Install

```bash
bash install.sh
```

Installs system deps, the SRE Node daemon, the Python package via `pipx`, two
Piper voices (`en_US-lessac-medium`, `fr_FR-siwis-medium`), a systemd user unit,
and a desktop entry.

## Use

- Select math text in any window.
- Press **Ctrl+Alt+M**.
- Listen.

Switch language via the **tray icon** (click → English / Français), or:

```bash
math-speak --lang fr
math-speak --lang en
```

## CLI

| Command | What it does |
|---|---|
| `math-speak` | Trigger the daemon to read the current selection |
| `math-speak --trigger` | Same |
| `math-speak --say "..."` | Synthesize the given text directly |
| `math-speak --lang en\|fr` | Set language (persists to config) |
| `math-speak --selftest` | Run the 8-case smoke matrix (text only) |
| `math-speakd --foreground` | Run the background daemon |

## Config

`~/.config/math-speak/config.toml`

```toml
hotkey = "<ctrl>+<alt>+m"
language = "en"                   # or "fr"
piper_voice_en = "en_US-lessac-medium"
piper_voice_fr = "fr_FR-siwis-medium"
espeak_fallback = true
llm_endpoint = "http://127.0.0.1:11434"
llm_model = "gemma3n:e2b"
llm_timeout_s = 3.0
model_dir = "~/.local/share/math-speak/models"
speed = 1.0
raw_mode = false                  # true = skip normalizer; espeak-ng raw read
```

## Hotkey on Wayland

`pynput.GlobalHotKeys` works under X11 and many Wayland compositors. If it
fails on a restrictive compositor (vanilla GNOME on Wayland), bind a custom
shortcut in your DE settings to:

```
math-speak --trigger
```

That sends a request over a UNIX socket to the running daemon.

## Files

- `src/math_speak/`  — Python package
- `node/`            — SRE daemon (Node 18+)
- `systemd/`         — user unit
- `install.sh`       — one-shot installer
- `tests/`           — pytest unit + integration
