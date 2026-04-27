#Requires -Version 5.1
<#
.SYNOPSIS
    Installs voice-prompt on Windows.

.PARAMETER FromSource
    Build from source using 'cargo tauri build' before installing.

.PARAMETER ReleaseUrl
    URL to a pre-built .msi installer to download and run.

.EXAMPLE
    .\install.ps1 -FromSource
    .\install.ps1 -ReleaseUrl "https://example.com/voice-prompt-0.2.0-x64.msi"
#>
param(
    [switch]$FromSource,
    [string]$ReleaseUrl
)

$ErrorActionPreference = 'Stop'
$tool    = 'voice-prompt'
$appDir  = Join-Path $env:LOCALAPPDATA $tool

Write-Host "=== voice-prompt installer ===" -ForegroundColor Cyan

# ---------------------------------------------------------------------------
# 1. Ensure WebView2 runtime is present
# ---------------------------------------------------------------------------
function Test-WebView2 {
    $regPaths = @(
        'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
        'HKCU:\Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
    )
    foreach ($p in $regPaths) {
        if (Test-Path $p) { return $true }
    }
    return $false
}

if (-not (Test-WebView2)) {
    Write-Host "[*] WebView2 runtime not found. Downloading bootstrapper..." -ForegroundColor Yellow
    $wv2Bootstrap = Join-Path $env:TEMP 'MicrosoftEdgeWebview2Setup.exe'
    $wv2Url = 'https://go.microsoft.com/fwlink/p/?LinkId=2124703'
    Invoke-WebRequest -Uri $wv2Url -OutFile $wv2Bootstrap -UseBasicParsing
    Write-Host "[*] Installing WebView2 runtime silently..."
    Start-Process -FilePath $wv2Bootstrap -ArgumentList '/silent /install' -Wait -NoNewWindow
    Write-Host "[+] WebView2 runtime installed." -ForegroundColor Green
} else {
    Write-Host "[+] WebView2 runtime already present." -ForegroundColor Green
}

# ---------------------------------------------------------------------------
# 2. Obtain the MSI
# ---------------------------------------------------------------------------
$msiPath = $null

if ($FromSource) {
    Write-Host "[*] Building from source with 'cargo tauri build'..."
    $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    Push-Location $scriptDir
    try {
        cargo tauri build
    } finally {
        Pop-Location
    }
    # Locate the freshly-built MSI
    $msiDir = Join-Path $scriptDir 'src-tauri\target\release\bundle\msi'
    $msiFiles = Get-ChildItem -Path $msiDir -Filter '*.msi' -ErrorAction SilentlyContinue
    if (-not $msiFiles) {
        Write-Error "[!] No .msi found in $msiDir after build. Check the Tauri build output."
    }
    # Pick the newest one in case there are multiple
    $msiPath = ($msiFiles | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
    Write-Host "[+] Built MSI: $msiPath" -ForegroundColor Green

} elseif ($ReleaseUrl) {
    Write-Host "[*] Downloading MSI from $ReleaseUrl ..."
    $msiPath = Join-Path $env:TEMP "$tool-installer.msi"
    Invoke-WebRequest -Uri $ReleaseUrl -OutFile $msiPath -UseBasicParsing
    Write-Host "[+] Downloaded to $msiPath" -ForegroundColor Green

} else {
    Write-Error "[!] Specify either -FromSource or -ReleaseUrl. Run: Get-Help .\install.ps1"
}

# ---------------------------------------------------------------------------
# 3. Install via msiexec
# ---------------------------------------------------------------------------
Write-Host "[*] Installing $tool to $appDir ..."
New-Item -ItemType Directory -Path $appDir -Force | Out-Null
$msiArgs = @('/i', $msiPath, '/quiet', "INSTALLDIR=`"$appDir`"", '/norestart')
$proc = Start-Process -FilePath 'msiexec.exe' -ArgumentList $msiArgs -Wait -PassThru -NoNewWindow
if ($proc.ExitCode -ne 0) {
    Write-Error "[!] msiexec exited with code $($proc.ExitCode). Installation may have failed."
}
Write-Host "[+] MSI installation complete." -ForegroundColor Green

# ---------------------------------------------------------------------------
# 4. Python check and venv setup
# ---------------------------------------------------------------------------
$pyCmd = $null
foreach ($candidate in @('py', 'python3', 'python')) {
    if (Get-Command $candidate -ErrorAction SilentlyContinue) {
        $pyCmd = $candidate
        break
    }
}

if (-not $pyCmd) {
    Write-Host ""
    Write-Host "[!] WARNING: Python not found on PATH." -ForegroundColor Yellow
    Write-Host "    voice-prompt requires Python with faster-whisper for transcription."
    Write-Host "    Download and install Python 3.10+ from: https://www.python.org/downloads/"
    Write-Host "    After installing Python, re-run this script to finish setup."
} else {
    Write-Host "[*] Python found: $pyCmd"
    $venvDir = Join-Path $appDir 'venv'
    if (-not (Test-Path $venvDir)) {
        Write-Host "[*] Creating Python virtual environment at $venvDir ..."
        & $pyCmd -m venv $venvDir
    } else {
        Write-Host "[*] Python venv already exists at $venvDir — reusing."
    }

    $pip = Join-Path $venvDir 'Scripts\pip.exe'
    Write-Host "[*] Installing faster-whisper into venv..."
    & $pip install --upgrade pip --quiet
    & $pip install faster-whisper --quiet
    Write-Host "[+] faster-whisper installed." -ForegroundColor Green

    # Copy daemon script into appDir
    $daemonSrc = Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) 'whisper_daemon.py'
    if (Test-Path $daemonSrc) {
        Copy-Item -Path $daemonSrc -Destination (Join-Path $appDir 'whisper_daemon.py') -Force
        Write-Host "[+] whisper_daemon.py copied to $appDir" -ForegroundColor Green
    } else {
        Write-Host "[!] WARNING: whisper_daemon.py not found at $daemonSrc — skipping." -ForegroundColor Yellow
    }
}

# ---------------------------------------------------------------------------
# 5. Create Start Menu shortcut
# ---------------------------------------------------------------------------
$startMenuDir = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
$shortcutPath = Join-Path $startMenuDir "$tool.lnk"
$exePath      = Join-Path $appDir "$tool.exe"

if (Test-Path $exePath) {
    $wsh = New-Object -ComObject WScript.Shell
    $shortcut = $wsh.CreateShortcut($shortcutPath)
    $shortcut.TargetPath       = $exePath
    $shortcut.WorkingDirectory = $appDir
    $shortcut.Description      = 'Push-to-talk speech-to-text transcription'
    $shortcut.Save()
    Write-Host "[+] Start Menu shortcut created at $shortcutPath" -ForegroundColor Green
} else {
    Write-Host "[!] WARNING: $exePath not found — Start Menu shortcut skipped." -ForegroundColor Yellow
    Write-Host "    The MSI may have installed the binary to a different location."
}

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "Installed. Launch from Start Menu." -ForegroundColor Cyan
