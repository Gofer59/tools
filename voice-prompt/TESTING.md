# voice-prompt — Manual Smoke Test Matrix

## Latency Measurement Methodology

Enable debug logging before running:
```bash
RUST_LOG=debug voice-prompt 2>&1 | grep -E "(hotkey released|tiny preview|large final|inject)"
```

Timestamps are printed for each stage. Measure from `hotkey released` to `tiny preview` for preview latency,
and from `hotkey released` to `large final` for total latency.

Example output:
```
[voice-prompt] hotkey released — finishing recording
[voice-prompt] tiny preview (287 ms): "bonjour le monde"
[voice-prompt] large final (1423 ms): "Bonjour le monde."
```

---

## Platform Matrix

### Linux X11

**Prerequisites:** xdotool installed (`apt install xdotool`)

| Test | Steps | Expected |
|------|-------|----------|
| Basic EN, preview=inline-replace | Open text editor. Hold Ctrl+Alt+Space, say "hello world", release. | Preview "hello world" appears ~200-400 ms. Replaced by large model output ~1-3 s later. No duplicate characters. |
| Basic FR, preview=inline-replace | Set language=fr in Settings. Hold Ctrl+Alt+Space, say "bonjour le monde", release. | Preview "bonjour le monde" appears. Replaced by "Bonjour le monde." (capitalized, accurate). |
| Preview disabled (none) | Set preview_mode=none. Hold Ctrl+Alt+Space, say "hello", release. | Single injection after large model completes. No preview. Byte-identical to pre-v0.3.0. |
| Overlay mode EN | Set preview_mode=overlay. Hold Ctrl+Alt+Space, say "testing one two three", release. | Floating overlay window shows preview text. No text in target app until large completes. Overlay hides, final text injected. |
| Language auto-detect | Set language=auto. Speak French, then English in separate presses. | Correct transcription in detected language each time. |
| Large model wins race | Use large=tiny (same model for tiny and large) to simulate race. | No duplicate text. One clean injection. |

### Linux Wayland

**Prerequisites:** wtype installed (`apt install wtype` or `pacman -S wtype`)

| Test | Steps | Expected |
|------|-------|----------|
| wtype detection | Check `$XDG_SESSION_TYPE == wayland`. Run voice-prompt. Speak a phrase. | Text injected via wtype (visible in target app). |
| Wayland missing wtype | Remove/rename wtype. Speak a phrase with preview=inline-replace. | Error logged: "wtype not found — install wtype". Text injection fails gracefully (no crash). |
| FR + Overlay on Wayland | Set preview_mode=overlay, language=fr. Speak French. | Overlay window appears with French preview. Final French text injected via wtype. |

**Wayland install note:** If wtype is unavailable, switch to `preview_mode=overlay` (no injection during preview phase) or install `ydotool` as an alternative and switch the injection command.

### Windows

**Prerequisites:** enigo crate compiled in (automatic via `cfg(windows)`)

| Test | Steps | Expected |
|------|-------|----------|
| Basic EN injection | Open Notepad. Hold Ctrl+Alt+Space, say "hello world", release. | Text appears via Windows SendInput (enigo). No xdotool references. |
| Preview inline-replace | Same with preview=inline-replace. | Preview text appears, then Backspace×n clears it, final text injected. |
| Cross-compile check | `cargo check --target x86_64-pc-windows-gnu` | Passes without errors. xdotool not referenced in Windows paths. |

**Note:** Full Windows runtime tests require a Windows machine. The cross-compile check verifies cfg-gating is correct.

---

## Regression Tests (run after every change)

1. **Zero regression with preview=none:** Set preview_mode=none. Run 5 EN phrases. Output must match pre-v0.3.0 behavior (single injection, ~200ms delay from hotkey release on tiny model).

2. **No duplication:** Run 10 phrases with preview=inline-replace. Inspect injected text — no doubled words or partial duplicates.

3. **FR + EN toggle:** Switch language mid-session. Both produce correctly-accented output.

4. **Config persistence:** Change tiny_model, preview_mode, close app, reopen. Settings preserved.

5. **Daemon restart:** Change whisper_model in Settings. Verify daemon-ready event fires, next transcription uses new model.

---

## Automated Tests

```bash
# Rust unit tests
cd src-tauri && cargo test

# Python tests (no model downloads, mocked)
python -m pytest python/tests/ -v

# Windows cross-compile check
cd src-tauri && cargo check --target x86_64-pc-windows-gnu
# Requires: rustup target add x86_64-pc-windows-gnu
#           apt install gcc-mingw-w64-x86-64 (MinGW cross-linker)
#
# NOTE: On this dev machine the conda-bundled CC is not a MinGW linker,
# so `cargo check --target x86_64-pc-windows-gnu` fails in the C build
# of the `aws-lc-sys` transitive dependency (pre-existing, not caused by
# this feature). The Windows-specific code paths (enigo, cfg(target_os="windows"))
# are syntactically verified by the host Linux build via cfg-attr parsing.
# Full Windows runtime tests require a Windows runner or wine cross environment.
```
