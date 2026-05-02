# Changelog

## [0.3.0] — 2026-05-01

### Added

- **Two-stage transcription:** a tiny Whisper model produces a preview injection within ~300 ms of hotkey release; the large model runs concurrently and replaces the preview with accurate output (~1-3 s). Both models read the same WAV file — no double-recording.
- **Preview modes** (configurable in Settings):
  - `inline-replace` (default): preview text is typed, then backspace-deleted and replaced by the large model result.
  - `overlay`: a floating always-on-top window shows the preview; no text is injected until the large model completes.
  - `none`: single-model behavior, byte-identical to v0.2.0.
- **Wayland injection** via `wtype` (runtime-detected from `$XDG_SESSION_TYPE`). Documents install requirement.
- **Windows injection** via `enigo` crate (`cfg(windows)` — xdotool never referenced on Windows).
- **Per-model device selection** in Settings: run tiny on CPU, large on GPU (CUDA) independently.
- New config fields: `tiny_model`, `preview_mode`, `tiny_device`, `large_device`.
- Overlay Tauri window (`label: "overlay"`) — decorations-free, transparent, always-on-top.
- Python pytest suite (`python/tests/`) — mocked, no model downloads required.
- `TESTING.md` — manual smoke test matrix for X11, Wayland, Windows; latency measurement guide.

### Changed

- Python daemon now accepts optional 4th CLI arg `device` (default `"cpu"`), enabling per-instance GPU selection.
- `capture_active_window()` returns `None` on Wayland/Windows (xdotool not available there).

### Fixed

- Stale `Left Meta+S` hotkey reference corrected to `Ctrl+Alt+Space` in root `CLAUDE.md`.

## [0.2.0] — 2025 (prior release)

Single-model push-to-talk STT, Tauri 2 + SvelteKit GUI, persistent Python Whisper daemon, model catalog with streaming download.
