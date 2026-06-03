param(
    [switch]$FromSource,
    [string]$ReleaseUrl
)

$ErrorActionPreference = 'Continue'

$tool    = 'voice-speak'
$appDir  = Join-Path $env:LOCALAPPDATA $tool
$binDir  = Join-Path $env:LOCALAPPDATA 'Microsoft\WindowsApps'
$startMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'

function Ensure-Python {
    # Returns a usable Python command name (py / python3 / python), installing
    # Python 3.12 first if no real interpreter is on PATH. Rejects the Microsoft
    # Store app-execution-alias stub: it lives under WindowsApps and exits with
    # "Python was not found" instead of running.
    foreach ($candidate in @('py','python3','python')) {
        $cmd = Get-Command $candidate -ErrorAction SilentlyContinue
        if ($cmd -and ($cmd.Source -notlike '*\WindowsApps\*')) {
            return $candidate
        }
    }

    Write-Host "[install] Python 3 not found on PATH. Installing..."

    if (Get-Command winget -ErrorAction SilentlyContinue) {
        Write-Host "[install] Installing Python 3.12 via winget (user scope)..."
        & winget install --id Python.Python.3.12 --scope user --silent `
            --accept-source-agreements --accept-package-agreements
    } else {
        Write-Host "[install] winget unavailable, downloading installer from python.org..."
        $pyUrl = 'https://www.python.org/ftp/python/3.12.7/python-3.12.7-amd64.exe'
        $pyExe = Join-Path $env:TEMP 'python-3.12.7-amd64.exe'
        Invoke-WebRequest -Uri $pyUrl -OutFile $pyExe -UseBasicParsing
        Write-Host "[install] Running python.org installer (silent, per-user, PATH on)..."
        Start-Process -FilePath $pyExe `
            -ArgumentList '/quiet','InstallAllUsers=0','PrependPath=1','Include_pip=1','Include_launcher=1' `
            -Wait
        Remove-Item $pyExe -Force
    }

    # Refresh PATH so the freshly installed python is visible in this session.
    $env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' +
                [Environment]::GetEnvironmentVariable('Path','User')

    foreach ($candidate in @('py','python3','python')) {
        $cmd = Get-Command $candidate -ErrorAction SilentlyContinue
        if ($cmd -and ($cmd.Source -notlike '*\WindowsApps\*')) {
            Write-Host "[install] Python installed: $($cmd.Source)"
            return $candidate
        }
    }

    Write-Error "Python install attempted but no usable interpreter on PATH. Install manually from https://www.python.org/downloads/ and re-run."
    return $null
}

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

$pyCmd = Ensure-Python
if (-not $pyCmd) {
    Write-Error "Cannot continue without Python. Aborting venv setup."
    exit 1
}

Write-Host "[install] Setting up Python venv at $appDir\venv …"
& $pyCmd -m venv "$appDir\venv"
& "$appDir\venv\Scripts\pip.exe" install --quiet --upgrade pip
& "$appDir\venv\Scripts\pip.exe" install --quiet piper-tts numpy
Write-Host "[install] Python dependencies installed."

# ── 4. Copy daemon script ────────────────────────────────────────────────────

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$daemonSrc = Join-Path $scriptDir 'python\piper_daemon.py'
Copy-Item -Path $daemonSrc -Destination "$appDir\piper_daemon.py" -Force
Write-Host "[install] Daemon script copied to $appDir\piper_daemon.py"

# Seed config.json with the absolute venv Python path so first launch works.
# Without this, the Rust default falls back to "python3", which on Windows
# hits the Microsoft Store app-execution-alias stub and crashes the daemon.
$configPath = Join-Path $appDir 'config.json'
if (-not (Test-Path $configPath)) {
    $venvPython = Join-Path $appDir 'venv\Scripts\python.exe'
    $defaultConfig = [ordered]@{
        hotkey        = 'Ctrl+Alt+V'
        voice         = 'en_US-lessac-medium'
        speed         = 1.0
        noise_scale   = 0.667
        noise_w_scale = 0.8
        python_bin    = $venvPython
    }
    $defaultConfig | ConvertTo-Json | Set-Content -Path $configPath -Encoding UTF8
    Write-Host "[install] Seeded $configPath with python_bin=$venvPython"
} else {
    Write-Host "[install] config.json already exists at $configPath — leaving untouched."
}

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
