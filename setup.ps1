<#
.SYNOPSIS
    Faraday - Setup Phase 0 (Windows 10+)
.DESCRIPTION
    Installe l'outillage Rust (rustup), prepare l'environnement de build CEF
    (CEF_PATH) puis compile Faraday.
#>
$ErrorActionPreference = "Stop"

Write-Host "=== Faraday - Setup Phase 0 ===" -ForegroundColor Cyan

# --- 1. Verifier / installer Rust ---
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "[1/3] Installation de Rust (rustup)..." -ForegroundColor Yellow
    $rustup = Join-Path $env:TEMP "rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustup
    & $rustup -y --no-modify-path
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
    Write-Host "Rust installe : $(cargo --version)" -ForegroundColor Green
} else {
    Write-Host "[1/3] Rust deja present : $(cargo --version)" -ForegroundColor Green
}

# --- 2. Definir CEF_PATH (cache des binaires Chromium/CEF) ---
Write-Host "[2/3] Configuration de CEF_PATH..." -ForegroundColor Yellow
$cefPath = Join-Path $env:USERPROFILE ".local\share\cef"
if (-not (Test-Path $cefPath)) {
    New-Item -ItemType Directory -Force -Path $cefPath | Out-Null
}
$env:CEF_PATH = $cefPath
[Environment]::SetEnvironmentVariable("CEF_PATH", $cefPath, "User")
Write-Host "CEF_PATH = $cefPath" -ForegroundColor Green

# --- 3. Build ---
Write-Host "[3/3] Build de Faraday (cargo build)..." -ForegroundColor Yellow
cargo build --package faraday

Write-Host ""
Write-Host "=== Termine ===" -ForegroundColor Cyan
Write-Host "Lancer : cargo run --package faraday" -ForegroundColor Green

