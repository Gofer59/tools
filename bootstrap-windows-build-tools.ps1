#Requires -Version 5.1
<#
.SYNOPSIS
    Installs the build-time toolchains (Rust, Node.js LTS, tauri-cli) needed
    to build any `-FromSource` target in this repo on a fresh Windows machine.

.DESCRIPTION
    Run this ONCE before invoking any tool's install.ps1 with -FromSource.
    Idempotent: skips anything already present unless -Force is supplied.

    Tries winget first; falls back to upstream installers if winget is absent
    (Windows 10 pre-1809) or fails.

    Runtime prerequisites (Python, WebView2) are handled by each tool's own
    install.ps1 — they belong with the runtime, not the dev toolchain.

.PARAMETER Force
    Reinstall a toolchain even if it is already detected.

.EXAMPLE
    .\bootstrap-windows-build-tools.ps1
    # Then in any tool dir:
    .\voice-prompt\install.ps1 -FromSource
#>
param([switch]$Force)

$ErrorActionPreference = 'Continue'

Write-Host "=== bootstrap: Windows build toolchains ===" -ForegroundColor Cyan

function Refresh-Path {
    $env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' +
                [Environment]::GetEnvironmentVariable('Path','User')
}

# ─── 1. Rust (rustup) ────────────────────────────────────────────────────────
function Install-Rust {
    if (-not $Force `
        -and (Get-Command rustc -ErrorAction SilentlyContinue) `
        -and (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host "[+] rustc + cargo already on PATH — skipping Rust install." -ForegroundColor Green
        return
    }

    if (Get-Command winget -ErrorAction SilentlyContinue) {
        Write-Host "[*] Installing Rust toolchain via winget (Rustlang.Rustup)..."
        & winget install --id Rustlang.Rustup --scope user --silent `
            --accept-source-agreements --accept-package-agreements
    } else {
        Write-Host "[*] winget unavailable, downloading rustup-init.exe..."
        $exe = Join-Path $env:TEMP 'rustup-init.exe'
        Invoke-WebRequest -Uri 'https://win.rustup.rs/x86_64' -OutFile $exe -UseBasicParsing
        & $exe -y --default-toolchain stable --profile minimal
        Remove-Item $exe -Force
    }

    # cargo lives at %USERPROFILE%\.cargo\bin even before a logout/login cycle.
    $cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
    if ($env:Path -notlike "*$cargoBin*") { $env:Path = "$cargoBin;$env:Path" }
    Refresh-Path

    if (Get-Command rustc -ErrorAction SilentlyContinue) {
        $v = (& rustc --version)
        Write-Host "[+] Rust ready: $v" -ForegroundColor Green
    } else {
        Write-Warning "Rust install attempted but rustc not on PATH. Open a new shell or install manually from https://rustup.rs"
    }
}

# ─── 2. Node.js 20+ LTS ──────────────────────────────────────────────────────
function Install-Node {
    if (-not $Force -and (Get-Command node -ErrorAction SilentlyContinue)) {
        $current = (& node --version) -replace '^v',''
        try {
            if ([version]$current -ge [version]'20.0.0') {
                Write-Host "[+] node v$current already installed — skipping." -ForegroundColor Green
                return
            }
            Write-Host "[*] node v$current is older than 20 LTS — upgrading."
        } catch {
            Write-Host "[*] Could not parse node version '$current' — reinstalling LTS."
        }
    }

    if (Get-Command winget -ErrorAction SilentlyContinue) {
        Write-Host "[*] Installing Node.js LTS via winget (OpenJS.NodeJS.LTS)..."
        & winget install --id OpenJS.NodeJS.LTS --scope user --silent `
            --accept-source-agreements --accept-package-agreements
    } else {
        Write-Host "[*] winget unavailable, downloading Node.js LTS .msi..."
        $msi = Join-Path $env:TEMP 'node-lts-x64.msi'
        Invoke-WebRequest -Uri 'https://nodejs.org/dist/v20.18.0/node-v20.18.0-x64.msi' `
                          -OutFile $msi -UseBasicParsing
        Start-Process msiexec.exe -ArgumentList '/i', $msi, '/quiet', '/norestart' -Wait
        Remove-Item $msi -Force
    }

    Refresh-Path

    if (Get-Command node -ErrorAction SilentlyContinue) {
        Write-Host "[+] Node.js ready: $(& node --version)" -ForegroundColor Green
    } else {
        Write-Warning "Node.js install attempted but node not on PATH. Open a new shell or install manually from https://nodejs.org"
    }
}

# ─── 3. tauri-cli (cargo plugin) ─────────────────────────────────────────────
function Install-TauriCli {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error "cargo not found — Rust install failed or PATH not refreshed. Open a new PowerShell and re-run."
        return
    }

    if (-not $Force) {
        $existing = & cargo install --list 2>$null | Select-String '^tauri-cli '
        if ($existing) {
            Write-Host "[+] tauri-cli already installed via cargo — skipping." -ForegroundColor Green
            return
        }
    }

    Write-Host "[*] Installing tauri-cli via cargo (this compiles from source — can take a few minutes)..."
    & cargo install tauri-cli --locked --version '^2.0'

    if ($LASTEXITCODE -ne 0) {
        Write-Warning "cargo install tauri-cli returned $LASTEXITCODE — check the cargo output above."
        return
    }
    Write-Host "[+] tauri-cli ready: $(& cargo tauri --version 2>&1)" -ForegroundColor Green
}

Install-Rust
Install-Node
Install-TauriCli

Write-Host ""
Write-Host "=== bootstrap done ===" -ForegroundColor Cyan
Write-Host "Verify in a fresh shell:"
Write-Host "    rustc --version"
Write-Host "    node --version"
Write-Host "    cargo tauri --version"
Write-Host ""
Write-Host "You can now run any tool's install.ps1 with -FromSource. For example:"
Write-Host "    cd voice-prompt"
Write-Host "    .\install.ps1 -FromSource"
