#Requires -Version 5.1
<#
  Faraday - Phase 5 : build Release + regroupement applicatif + zip portable.
  (Fichier en ASCII pur : les accents cassent le parsing PS 5.1 sans BOM.)

  Usage :
    .\packaging\build-release.ps1                     # build + dist + zip portable
    .\packaging\build-release.ps1 -Sandbox            # active le sandbox Chromium (cef/sandbox)
    .\packaging\build-release.ps1 -SkipBuild          # regroupe seulement (dist deja present)
    .\packaging\build-release.ps1 -Inno               # tente aussi l'installateur Inno (ISCC requis)
    .\packaging\build-release.ps1 -CertPath my.pfx -CertPass "***"   # signe les .exe apres build

  Pre-requis : Rust stable (MSVC) + CMake + Ninja, variable CEF_PATH positionnee.
#>
[CmdletBinding()]
param(
    [switch]$Sandbox,
    [switch]$SkipBuild,
    [switch]$Inno,
    [string]$CertPath = "",
    [string]$CertPass = ""
)

$ErrorActionPreference = "Stop"

# --- Environnement CEF / cargo ---------------------------------------------
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
if (-not $env:CEF_PATH) { $env:CEF_PATH = "$env:USERPROFILE\.local\share\cef" }
if (-not (Test-Path $env:CEF_PATH)) {
    Write-Error "CEF_PATH introuvable ($env:CEF_PATH). Lance d'abord un cargo build en dev."
}

$root    = Split-Path -Parent $PSScriptRoot          # racine du depot
$profile = @()
if ($Sandbox) { $profile = @("--features", "sandbox") }
$version = "0.1.0"
try {
    $m = Select-String -Path "$root\Cargo.toml" -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if ($m) { $version = $m.Matches[0].Groups[1].Value }
} catch { }

# --- 1) Build Release --------------------------------------------------------
if (-not $SkipBuild) {
    Write-Host "==> cargo build --release $($profile -join ' ') (peut prendre plusieurs minutes)"
    Push-Location $root
    try {
        # Les DEUX binaires : le navigateur + le helper de processus CEF.
        cargo build --release --package faraday --bin faraday --bin faraday_helper @profile
        if ($LASTEXITCODE -ne 0) { throw "cargo build a echoue (code $LASTEXITCODE)" }
    } finally { Pop-Location }
}
$buildDir = "$root\target\release"
if (-not (Test-Path $buildDir)) { Write-Error "Repertoire target\release absent - build requis." }

# --- 2) Regroupement dans dist\Faraday ---------------------------------------
$stage   = "$root\dist\Faraday"
$distOut = "$root\dist"
New-Item -ItemType Directory -Force -Path $distOut | Out-Null
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force -Path $stage | Out-Null

# Fichiers runtime requis (binaires Faraday + CEF). bootstrap*.exe servent au
# sandbox Chromium (inclus pour compatibilite, inoffensifs sinon).
$files = @(
    "faraday.exe", "faraday_helper.exe",
    "libcef.dll", "chrome_elf.dll", "libEGL.dll", "libGLESv2.dll",
    "d3dcompiler_47.dll", "dxcompiler.dll", "dxil.dll",
    "icudtl.dat", "resources.pak",
    "chrome_100_percent.pak", "chrome_200_percent.pak",
    "v8_context_snapshot.bin",
    "vk_swiftshader.dll", "vk_swiftshader_icd.json", "vulkan-1.dll",
    "CREDITS.html", "bootstrap.exe", "bootstrapc.exe"
)
$missing = @()
foreach ($f in $files) {
    $src = Join-Path $buildDir $f
    if (Test-Path $src) { Copy-Item $src -Destination $stage }
    else { $missing += $f }
}
# Dossier locales (obligatoire : traductions Chromium).
$locales = Join-Path $buildDir "locales"
if (Test-Path $locales) { Copy-Item $locales -Destination $stage -Recurse }
else { $missing += "locales\" }

if ($missing.Count -gt 0) {
    Write-Warning "Fichiers attendus absents de target\release : $($missing -join ', ')"
}
$sizeMb = [math]::Round((Get-ChildItem $stage -Recurse -File | Measure-Object Length -Sum).Sum / 1MB, 1)
Write-Host "==> App regroupee dans : $stage"
Write-Host "    (taille : $sizeMb Mo)"

# --- 3) Zip portable ----------------------------------------------------------
$zip = Join-Path $distOut "Faraday-$version-x64-portable.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Write-Host "==> Creation du zip portable : $zip"
Compress-Archive -Path "$stage\*" -DestinationPath $zip -CompressionLevel Optimal
$zipMb = [math]::Round((Get-Item $zip).Length / 1MB, 1)
Write-Host "    (zip : $zipMb Mo)"

# --- 4) Signature optionnelle (SignTool) --------------------------------------
if ($CertPath) {
    if (-not (Get-Command signtool.exe -ErrorAction SilentlyContinue)) {
        Write-Warning "signtool.exe introuvable - signature ignoree."
    } else {
        if ($CertPass) {
            $certArg = @("/f", $CertPath, "/p", $CertPass)
        } else {
            $certArg = @("/f", $CertPath)
        }
        Get-ChildItem $stage -Filter *.exe | ForEach-Object {
            Write-Host "==> Signature : $($_.Name)"
            & signtool.exe sign /fd SHA256 @certArg "/tr http://timestamp.digicert.com" "/td SHA256" $_.FullName
        }
        & signtool.exe sign /fd SHA256 @certArg "/tr http://timestamp.digicert.com" "/td SHA256" $zip
    }
}

# --- 5) Installateur Inno Setup (si demande et disponible) ---------------------
if ($Inno) {
    $iscc = (Get-Command ISCC.exe -ErrorAction SilentlyContinue).Source
    if (-not $iscc) {
        $cand = "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
        if (Test-Path $cand) { $iscc = $cand }
    }
    if (-not $iscc) {
        Write-Warning "ISCC.exe (Inno Setup 6) introuvable - installe-le puis relance avec -Inno."
    } else {
        Write-Host "==> Compilation installateur Inno ..."
        & $iscc "$PSScriptRoot\faraday.iss"
        if ($LASTEXITCODE -ne 0) { throw "Inno Setup a echoue (code $LASTEXITCODE)" }
    }
}

Write-Host ""
Write-Host "TERMINE. Sorties :"
Write-Host "  - dossier app  : $stage"
Write-Host "  - zip portable : $zip"
if ($Inno) { Write-Host "  - installateur : $distOut\Faraday-Setup-$version-x64.exe" }
