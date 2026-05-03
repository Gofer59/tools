# Release Pipeline Design

**Date:** 2026-05-03
**Repo:** github.com/Gofer59/tools
**Status:** Approved

## Goal

Allow end users to download pre-built binaries for any single tool without cloning the repo or installing a Rust toolchain. Each tool releases independently via a GitHub Actions workflow triggered by a git tag.

## Tools in scope

voice-prompt, voice-speak, screen-ocr, deck-reader, threshold-filter, key-detect

## 1. Tag convention

Pattern: `<tool>-v<major>.<minor>.<patch>`

Examples:
```
voice-prompt-v1.0.0
voice-speak-v1.0.0
screen-ocr-v0.3.1
threshold-filter-v2.1.0
```

One tag → one release for that tool only. Other tools are untouched.

## 2. Workflow trigger & tag parsing

File: `.github/workflows/release.yml`

Trigger:
```yaml
on:
  push:
    tags: ['*-v*.*.*']
```

Tag parsing (shell):
```bash
TAG="${GITHUB_REF_NAME}"                      # e.g. voice-prompt-v1.0.0
VERSION="${TAG##*-v}"                         # 1.0.0
TOOL="${TAG%-v*}"                             # voice-prompt
```

## 3. Build matrix

Two parallel jobs per release:

| Job | Target triple | Method |
|-----|--------------|--------|
| build-x86_64 | x86_64-unknown-linux-gnu | `cargo build --release` on `ubuntu-latest` |
| build-aarch64 | aarch64-unknown-linux-gnu | `cross build --release` on `ubuntu-latest` |

Each job:
1. Checks out repo
2. Navigates to source dir (`linux/<tool>/` or `steamdeck/<tool>/` for deck-reader)
3. Builds binary
4. Uploads binary as workflow artifact named `binary-<arch>`

`cross` installed via `cargo install cross` (cached between runs).

## 4. Tarball contents

Each release publishes two assets:
```
<tool>-<version>-x86_64-linux.tar.gz
<tool>-<version>-aarch64-linux.tar.gz
```

Tarball layout:
```
<tool>-<version>/
  <tool>          ← compiled binary
  install.sh      ← end-user installer (downloads models at install time)
  config.toml     ← template config with comments
  python/         ← Python scripts (voice-prompt, voice-speak, screen-ocr, deck-reader only)
  README.md
```

No models bundled. `install.sh` downloads them on first install.

## 5. install.sh model download strategy

The release `install.sh` differs from the source-build `install.sh`:
- Copies pre-built binary instead of running `cargo build`
- Downloads models via `curl` instead of copying from `models/`

Per-tool model sources:

| Tool | Model | Source |
|------|-------|--------|
| voice-prompt | faster-whisper (small) | auto-downloaded by faster-whisper on first run via HuggingFace |
| voice-speak | Piper en_US-lessac-medium + fr_FR-siwis-medium | github.com/rhasspy/piper/releases |
| screen-ocr | Piper (same) + tessdata eng+fra | piper releases + github.com/tesseract-ocr/tessdata |
| deck-reader | Piper + tessdata | same as screen-ocr |
| threshold-filter | none | — |
| key-detect | none | — |

Download rules in `install.sh`:
- `curl -L --fail` with progress bar
- Skip download if model file already present (idempotent)
- Print size estimate before each download

## 6. Release creation job

After both build jobs succeed, a `create-release` job:
1. Downloads both binary artifacts
2. Assembles tarball contents from repo (`install.sh`, `config.toml`, `python/`, `README.md`)
3. Packages `<tool>-<version>-x86_64-linux.tar.gz` and `<tool>-<version>-aarch64-linux.tar.gz`
4. Runs `gh release create`:
   - Tag: as pushed
   - Title: `<tool> v<version>`
   - Body: README first section + install snippet (see below)
   - Assets: both tarballs

## 7. Release body install snippet

```bash
# Detect arch and download
ARCH=$(uname -m)
curl -LO https://github.com/Gofer59/tools/releases/download/<tool>-v<version>/<tool>-<version>-${ARCH}-linux.tar.gz
tar xzf <tool>-<version>-${ARCH}-linux.tar.gz
cd <tool>-<version>
bash install.sh
```

## 8. File locations

| File | Path in repo |
|------|-------------|
| Workflow | `.github/workflows/release.yml` |
| Source dirs | `linux/<tool>/` (most tools), `steamdeck/deck-reader/` |
| Release install.sh | `linux/<tool>/install-release.sh` (new file, distinct from source-build install.sh) |

The source-build `install.sh` (builds from source, for developers) is kept as-is. The release `install-release.sh` is what gets packaged into the tarball and renamed to `install.sh` at packaging time.
