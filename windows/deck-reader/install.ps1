<#
.SYNOPSIS
    Install deck-reader on Windows 10/11.

.DESCRIPTION
    Builds the Rust binary, installs Tesseract via winget, creates a Python
    venv, downloads the default Piper voice model, writes a default config,
    and creates a Start Menu shortcut.

    Run from an elevated PowerShell session:

        Set-ExecutionPolicy Bypass -Scope Process
        .\install.ps1

    Or use the one-liner wrapper:

        .\install.bat

.PARAMETER SkipModel
    Skip downloading the Piper voice model (useful if already downloaded or
    on a slow connection — download manually later).

.NOTES
    Requirements (must be pre-installed):
        - Python 3.10+ (python.exe in PATH)
        - Rust / cargo   (cargo.exe in PATH)
        - winget         (Windows 10 1809+ / Windows 11)
#>

[CmdletBinding()]
param(
    [switch]$SkipModel
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# ─────────────────────────────────────────────────────────────────────────────
# Paths
# ─────────────────────────────────────────────────────────────────────────────
$ScriptDir = $PSScriptRoot
$DataDir   = "$env:LOCALAPPDATA\deck-reader"
$VenvDir   = "$DataDir\venv"
$ModelsDir = "$DataDir\models"
$BinDir    = "$DataDir\bin"
$PyDir     = "$DataDir\python"
$ConfigDir = "$env:APPDATA\deck-reader"
$StartMenu = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"

$VoiceName = "en_US-lessac-medium"
$VoiceBase = "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/lessac/medium"

# ─────────────────────────────────────────────────────────────────────────────
# Helper functions
# ─────────────────────────────────────────────────────────────────────────────
function Step($n, $msg) {
    Write-Host ""
    Write-Host "[$n] $msg" -ForegroundColor Cyan
}

function Ok($msg)   { Write-Host "    OK: $msg"    -ForegroundColor Green }
function Warn($msg) { Write-Host "    WARN: $msg"  -ForegroundColor Yellow }
function Fail($msg) {
    Write-Host ""
    Write-Host "FATAL: $msg" -ForegroundColor Red
    exit 1
}

function Require-Command($cmd, $installHint) {
    if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
        Fail "$cmd not found. $installHint"
    }
}

# ─────────────────────────────────────────────────────────────────────────────
# Step 1: Prerequisites
# ─────────────────────────────────────────────────────────────────────────────
Step 1 "Checking prerequisites"

Require-Command python  "Install Python 3.10+ from https://www.python.org/downloads/"
Require-Command cargo   "Install Rust from https://rustup.rs/"
Require-Command winget  "winget is required. Update via Microsoft Store (App Installer) or upgrade to Windows 10 1809+."

$pyVer = python --version 2>&1
Ok "Python: $pyVer"

$cargoVer = cargo --version 2>&1
Ok "Cargo: $cargoVer"

# ─────────────────────────────────────────────────────────────────────────────
# Step 2: Tesseract OCR
# ─────────────────────────────────────────────────────────────────────────────
Step 2 "Installing Tesseract OCR"

$tesseractExe = "C:\Program Files\Tesseract-OCR\tesseract.exe"
if (Test-Path $tesseractExe) {
    Ok "Tesseract already installed at $tesseractExe"
} else {
    Write-Host "    Installing via winget…"
    winget install --id UB-Mannheim.TesseractOCR `
        --accept-package-agreements `
        --accept-source-agreements `
        --silent
    if (-not (Test-Path $tesseractExe)) {
        Fail "Tesseract install failed. Try manually: winget install UB-Mannheim.TesseractOCR"
    }
    Ok "Tesseract installed."
}

# Add to session PATH so cargo build can find it if needed.
$tessDir = "C:\Program Files\Tesseract-OCR"
if ($env:PATH -notlike "*Tesseract*") {
    $env:PATH = "$tessDir;$env:PATH"
}

# ─────────────────────────────────────────────────────────────────────────────
# Step 3: Build the Rust binary
# ─────────────────────────────────────────────────────────────────────────────
Step 3 "Building deck-reader (cargo build --release)"

Push-Location $ScriptDir
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { Fail "cargo build --release failed." }
} finally {
    Pop-Location
}

$builtExe = "$ScriptDir\target\release\deck-reader.exe"
if (-not (Test-Path $builtExe)) {
    Fail "Binary not found after build: $builtExe"
}
Ok "Built: $builtExe"

# ─────────────────────────────────────────────────────────────────────────────
# Step 4: Copy binary and helper scripts to %LOCALAPPDATA%\deck-reader\bin\
# ─────────────────────────────────────────────────────────────────────────────
Step 4 "Copying binary and wrapper scripts to $BinDir"

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

Copy-Item -Force $builtExe "$BinDir\deck-reader.exe"
Ok "Copied deck-reader.exe"

# OCR wrapper: calls the venv Python with ocr_extract.py (mirrors Linux's .sh wrapper).
$ocrBat = "$BinDir\ocr_extract_wrapper.bat"
@"
@echo off
"%LOCALAPPDATA%\deck-reader\venv\Scripts\python.exe" "%LOCALAPPDATA%\deck-reader\python\ocr_extract.py" %*
"@ | Set-Content -Encoding ASCII $ocrBat
Ok "Created ocr_extract_wrapper.bat"

# TTS wrapper: unused on Windows (spawn_tts_fallback builds the path internally),
# but created for consistency with the Linux layout.
$ttsBat = "$BinDir\tts_speak_wrapper.bat"
@"
@echo off
"%LOCALAPPDATA%\deck-reader\venv\Scripts\python.exe" "%LOCALAPPDATA%\deck-reader\python\tts_speak.py" %*
"@ | Set-Content -Encoding ASCII $ttsBat
Ok "Created tts_speak_wrapper.bat"

# ─────────────────────────────────────────────────────────────────────────────
# Step 5: Python venv and dependencies
# ─────────────────────────────────────────────────────────────────────────────
Step 5 "Creating Python venv at $VenvDir"

New-Item -ItemType Directory -Force -Path $DataDir | Out-Null

if (-not (Test-Path "$VenvDir\Scripts\python.exe")) {
    python -m venv $VenvDir
    Ok "venv created."
} else {
    Ok "venv already exists."
}

Write-Host "    Installing Python dependencies (pip install -r requirements.txt)…"
& "$VenvDir\Scripts\pip.exe" install -r "$ScriptDir\requirements.txt" --quiet
if ($LASTEXITCODE -ne 0) { Fail "pip install failed." }
Ok "Python dependencies installed."

# ─────────────────────────────────────────────────────────────────────────────
# Step 6: Copy Python scripts
# ─────────────────────────────────────────────────────────────────────────────
Step 6 "Copying Python scripts to $PyDir"

New-Item -ItemType Directory -Force -Path $PyDir | Out-Null
Copy-Item -Force "$ScriptDir\python\*.py" $PyDir
Ok "Python scripts copied."

# ─────────────────────────────────────────────────────────────────────────────
# Step 7: Download Piper voice model
# ─────────────────────────────────────────────────────────────────────────────
Step 7 "Downloading Piper voice model ($VoiceName)"

New-Item -ItemType Directory -Force -Path $ModelsDir | Out-Null

if ($SkipModel) {
    Warn "Skipping model download (-SkipModel flag set)."
    Warn "Download manually: $VoiceBase/$VoiceName.onnx → $ModelsDir\$VoiceName.onnx"
    Warn "                   $VoiceBase/$VoiceName.onnx.json → $ModelsDir\$VoiceName.onnx.json"
} elseif (Test-Path "$ModelsDir\$VoiceName.onnx") {
    Ok "Model already present: $ModelsDir\$VoiceName.onnx"
} else {
    Write-Host "    Downloading $VoiceName.onnx (~65 MB)…"
    Invoke-WebRequest "$VoiceBase/$VoiceName.onnx" `
        -OutFile "$ModelsDir\$VoiceName.onnx" `
        -UseBasicParsing
    Write-Host "    Downloading $VoiceName.onnx.json…"
    Invoke-WebRequest "$VoiceBase/$VoiceName.onnx.json" `
        -OutFile "$ModelsDir\$VoiceName.onnx.json" `
        -UseBasicParsing
    Ok "Model downloaded."
}

# ─────────────────────────────────────────────────────────────────────────────
# Step 8: Config and Start Menu shortcut
# ─────────────────────────────────────────────────────────────────────────────
Step 8 "Writing config and Start Menu shortcut"

New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null

$configFile = "$ConfigDir\config.toml"
if (Test-Path $configFile) {
    Ok "Config already exists — not overwriting: $configFile"
} else {
    # Embed Windows-safe paths with forward slashes (shellexpand handles them).
    $modelsPath = $ModelsDir.Replace('\', '/')
    $venvPath   = $VenvDir.Replace('\', '/')
    $regionPath = ($DataDir + '\last_region.json').Replace('\', '/')

    @"
[hotkeys]
# Key names: MetaLeft, KeyQ, F9, AltGr, ControlLeft, etc.
# Combos: "Alt+KeyU"  or  "ControlLeft+F9"
# Run `deck-reader --detect-keys` to discover keycodes.
tts_toggle  = "Alt+KeyY"
ocr_select  = "Alt+KeyU"
ocr_capture = "Alt+KeyI"

[tts]
voice = "$VoiceName"    # Piper model name (must exist in models dir)
speed = 1.0              # 1.0=normal, 1.5=faster, 0.8=slower

[ocr]
language      = "eng"        # Tesseract lang codes: "eng", "eng+jpn", etc.
delivery_mode = "clipboard"  # Windows MVP: "clipboard" only ("type"/"both" not supported)
cleanup       = true         # clean OCR artifacts (stray symbols, repeated punct)

[paths]
models_dir  = "$modelsPath"
venv_dir    = "$venvPath"
region_file = "$regionPath"
"@ | Set-Content -Encoding UTF8 $configFile
    Ok "Config written: $configFile"
}

# Start Menu shortcut
$shortcutPath = "$StartMenu\deck-reader.lnk"
$shell        = New-Object -ComObject WScript.Shell
$shortcut     = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath       = "$BinDir\deck-reader.exe"
$shortcut.WorkingDirectory = $BinDir
$shortcut.Description      = "deck-reader — screen OCR + TTS hotkeys"
$shortcut.Save()
Ok "Start Menu shortcut: $shortcutPath"

# ─────────────────────────────────────────────────────────────────────────────
# Done
# ─────────────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "╔══════════════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║           deck-reader installed successfully          ║" -ForegroundColor Green
Write-Host "╠══════════════════════════════════════════════════════╣" -ForegroundColor Green
Write-Host "║  Binary   : $BinDir\deck-reader.exe" -ForegroundColor Green
Write-Host "║  Config   : $ConfigDir\config.toml" -ForegroundColor Green
Write-Host "╠══════════════════════════════════════════════════════╣" -ForegroundColor Green
Write-Host "║  Hotkeys (default):                                   ║" -ForegroundColor Green
Write-Host "║    Alt + U  →  select region + OCR                    ║" -ForegroundColor Green
Write-Host "║    Alt + I  →  re-OCR last region                     ║" -ForegroundColor Green
Write-Host "║    Alt + Y  →  toggle TTS (speak / stop)              ║" -ForegroundColor Green
Write-Host "╠══════════════════════════════════════════════════════╣" -ForegroundColor Green
Write-Host "║  Launch from Start Menu or run:                       ║" -ForegroundColor Green
Write-Host "║    $BinDir\deck-reader.exe" -ForegroundColor Green
Write-Host "╠══════════════════════════════════════════════════════╣" -ForegroundColor Green
Write-Host "║  NOTE: Windows Defender / SmartScreen may show a      ║" -ForegroundColor Yellow
Write-Host "║  warning on first run (unsigned binary). Right-click  ║" -ForegroundColor Yellow
Write-Host "║  the .exe → Properties → Unblock, or run from a      ║" -ForegroundColor Yellow
Write-Host "║  terminal to bypass SmartScreen.                      ║" -ForegroundColor Yellow
Write-Host "╚══════════════════════════════════════════════════════╝" -ForegroundColor Green
