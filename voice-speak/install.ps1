param(
    [switch]$FromSource,
    [string]$ReleaseUrl
)

$ErrorActionPreference = 'Stop'

$tool    = 'voice-speak'
$appDir  = Join-Path $env:LOCALAPPDATA $tool
$binDir  = Join-Path $env:LOCALAPPDATA 'Microsoft\WindowsApps'
$startMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'

# ── 1. Ensure WebView2 runtime is present ────────────────────────────────────

$webView2Key = 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
if (-not (Test-Path $webView2Key)) {
    Write-Host "[install] WebView2 runtime not found. Downloading installer…"
    $wv2Installer = Join-Path $env:TEMP 'MicrosoftEdgeWebview2Setup.exe'
    Invoke-WebRequest -Uri 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' `
                      -OutFile $wv2Installer -UseBasicParsing
    Start-Process -FilePath $wv2Installer -ArgumentList '/silent /install' -Wait
    Remove-Item $wv2Installer -Force
    Write-Host "[install] WebView2 runtime installed."
} else {
    Write-Host "[install] WebView2 runtime already present."
}

# ── 2. Install the voice-speak binary / MSI ──────────────────────────────────

New-Item -ItemType Directory -Force -Path $appDir | Out-Null

if ($FromSource) {
    # Build path relative to this script's location
    $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    $msiPath   = Join-Path $scriptDir "src-tauri\target\release\bundle\msi\voice-speak_*.msi"
    $msiFiles  = Resolve-Path $msiPath -ErrorAction SilentlyContinue
    if (-not $msiFiles) {
        Write-Error "MSI not found at $msiPath — run 'cargo tauri build' first."
        exit 1
    }
    $msiFile = ($msiFiles | Select-Object -First 1).Path
    Write-Host "[install] Installing from local MSI: $msiFile"
    Start-Process msiexec.exe -ArgumentList "/i `"$msiFile`" /qn" -Wait
}
elseif ($ReleaseUrl) {
    $msiDest = Join-Path $env:TEMP "$tool-install.msi"
    Write-Host "[install] Downloading MSI from $ReleaseUrl …"
    Invoke-WebRequest -Uri $ReleaseUrl -OutFile $msiDest -UseBasicParsing
    Write-Host "[install] Installing downloaded MSI…"
    Start-Process msiexec.exe -ArgumentList "/i `"$msiDest`" /qn" -Wait
    Remove-Item $msiDest -Force
}
else {
    Write-Error "Specify -FromSource to install from a local build, or -ReleaseUrl <url> to download an MSI."
    exit 1
}

# ── 3. Set up Python venv and install piper-tts ──────────────────────────────

Write-Host "[install] Setting up Python venv at $appDir\venv …"
python -m venv "$appDir\venv"
& "$appDir\venv\Scripts\pip.exe" install --quiet --upgrade pip
& "$appDir\venv\Scripts\pip.exe" install --quiet piper-tts numpy
Write-Host "[install] Python dependencies installed."

# ── 4. Copy daemon script ────────────────────────────────────────────────────

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$daemonSrc = Join-Path $scriptDir 'python\piper_daemon.py'
Copy-Item -Path $daemonSrc -Destination "$appDir\piper_daemon.py" -Force
Write-Host "[install] Daemon script copied to $appDir\piper_daemon.py"

# ── 5. Create Start Menu shortcut ────────────────────────────────────────────

New-Item -ItemType Directory -Force -Path $startMenu | Out-Null

# Locate the installed exe (MSI typically installs to Program Files)
$exeCandidates = @(
    "${env:ProgramFiles}\voice-speak\voice-speak.exe",
    "${env:ProgramFiles(x86)}\voice-speak\voice-speak.exe",
    (Join-Path $appDir 'voice-speak.exe')
)
$exePath = $exeCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1

if ($exePath) {
    $shell    = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut("$startMenu\Voice Speak.lnk")
    $shortcut.TargetPath       = $exePath
    $shortcut.WorkingDirectory = Split-Path $exePath
    $shortcut.Description      = 'TTS for highlighted text'
    $shortcut.Save()
    Write-Host "[install] Start Menu shortcut created."
} else {
    Write-Warning "Could not locate voice-speak.exe — Start Menu shortcut skipped."
}

Write-Host ""
Write-Host "Installed. Run 'voice-speak' or launch from the Start Menu."
