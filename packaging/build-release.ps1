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
$version = "0.1.0"
try {
    $m = Select-String -Path "$root\Cargo.toml" -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if ($m) { $version = $m.Matches[0].Groups[1].Value }
} catch { }

# --- 1) Build Release --------------------------------------------------------
if (-not $SkipBuild) {
    $mode = if ($Sandbox) { ' (mode sandbox : DLL + bootstrap)' } else { '' }
    Write-Host "==> cargo build --release$mode (peut prendre plusieurs minutes)"
    Push-Location $root
    try {
        # Binaires (navigateur + helper) ET bibliotheque (DLL pour le sandbox).
        cargo build --release --package faraday --bin faraday --bin faraday_helper --lib
        if ($LASTEXITCODE -ne 0) { throw "cargo build a echoue (code $LASTEXITCODE)" }
    } finally { Pop-Location }
}
$buildDir = "$root\target\release"
if (-not (Test-Path $buildDir)) { Write-Error "Repertoire target\release absent - build requis." }

# --- 2) Regroupement dans dist\Faraday ---------------------------------------
$stage   = "$root\dist\Faraday"
$distOut = "$root\dist"
New-Item -ItemType Directory -Force -Path $distOut | Out-Null
if (Test-Path $stage) {
    # Erreur explicite si l'application tourne encore (fichiers verrouilles).
    try {
        Remove-Item $stage -Recurse -Force -ErrorAction Stop
    } catch {
        Write-Error ("Impossible de vider '$stage' : des fichiers sont verrouilles.`n" +
                     "Ferme Faraday (et la fenetre Windows Sandbox si elle est ouverte) puis relance.")
    }
}
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

# --- 2b) Runtime Visual C++ (deploiement app-local) --------------------------
# faraday.exe / faraday_helper.exe importent VCRUNTIME140.dll. Sans VC++
# Redistributable installe (VM/Sandbox/machine propre), l'app ne demarrerait pas
# ("VCRUNTIME140.dll est introuvable"). On embarque le runtime a cote de l'exe ;
# c'est un deploiement "app-local" autorise par la licence redistribuable MS.
$crtNames = @("vcruntime140.dll", "vcruntime140_1.dll")
$crtDirs = @()
foreach ($r in @("C:\Program Files\Microsoft Visual Studio\2022", "C:\Program Files (x86)\Microsoft Visual Studio\2022")) {
    if (Test-Path $r) {
        $crtDirs += (Get-ChildItem $r -Recurse -Directory -Filter "Microsoft.VC143.CRT" -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match '\\x64\\' } | Select-Object -ExpandProperty FullName)
    }
}
$crtDirs += "C:\Windows\System32"   # repli
foreach ($n in $crtNames) {
    $src = $null
    foreach ($dir in $crtDirs) {
        $cand = Join-Path $dir $n
        if (Test-Path $cand) { $src = $cand; break }
    }
    if ($src) { Copy-Item $src -Destination $stage -Force }
    else { $missing += $n }
}

if ($missing.Count -gt 0) {
    Write-Warning "Fichiers attendus absents de target\release : $($missing -join ', ')"
}

# Verification : les dependances app-local doivent etre presentes dans le packaging.
foreach ($must in @("libcef.dll", "vcruntime140.dll")) {
    if (-not (Test-Path (Join-Path $stage $must))) {
        Write-Error "Dependance runtime manquante dans le packaging : $must"
    }
}

# --- 2c) Mode sandbox : bootstrap.exe (CEF) + faraday.dll --------------------
# Depuis Chromium M138 le sandbox Windows exige une application compilee en DLL
# (export `RunWinMain`) lancee par `bootstrap.exe`, qui fournit le sandbox_info.
if ($Sandbox) {
    $boot = Join-Path $buildDir "bootstrap.exe"
    $dll  = Join-Path $buildDir "faraday_core.dll"
    if (-not (Test-Path $boot)) { Write-Error "bootstrap.exe absent de target\release (requis pour le sandbox)." }
    if (-not (Test-Path $dll))  { Write-Error "faraday_core.dll absent - le build --lib a-t-il reussi ?" }
    Copy-Item $boot (Join-Path $stage "faraday.exe") -Force
    Copy-Item $dll  (Join-Path $stage "faraday.dll") -Force
    Write-Host "==> Mode SANDBOX : faraday.exe = bootstrap.exe (CEF), application dans faraday.dll"
} else {
    Write-Host "==> Mode classique (sandbox desactive) : faraday.exe = application"
}

$sizeMb = [math]::Round((Get-ChildItem $stage -Recurse -File | Measure-Object Length -Sum).Sum / 1MB, 1)
Write-Host "==> App regroupee dans : $stage"
Write-Host "    (taille : $sizeMb Mo)"

# --- 3) Signature des binaires (AVANT zip et installateur) -------------------
# On signe faraday.exe / faraday_helper.exe dans le dossier regroupe AVANT de
# creer le zip et l'installateur, afin que ces derniers contiennent bien les
# binaires signes. L'installateur sera signe apres sa compilation (section 5).
$signtool = $null
if ($CertPath) {
    $signtool = (Get-Command signtool.exe -ErrorAction SilentlyContinue).Source
    if (-not $signtool) {
        $cand = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin" -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match '\\x64\\' } | Sort-Object FullName -Descending | Select-Object -First 1
        if ($cand) { $signtool = $cand.FullName }
    }
    if (-not $signtool) {
        Write-Warning "signtool.exe introuvable (installe le Windows SDK) - signature ignoree."
        $CertPath = ""
    } elseif (-not (Test-Path $CertPath)) {
        Write-Warning "Certificat introuvable : $CertPath - signature ignoree."
        $CertPath = ""
        $signtool = $null
    } else {
        Write-Host "==> signtool : $signtool"
        Write-Host "==> certificat : $CertPath"
    }
}

function Invoke-FaradaySign {
    param([string]$Path)
    if (-not $signtool) { return }
    $certArg = @("/f", $CertPath)
    if ($CertPass) { $certArg += @("/p", $CertPass) }
    Write-Host "==> Signature : $(Split-Path $Path -Leaf)"
    & $signtool sign /fd SHA256 @certArg /tr http://timestamp.digicert.com /td SHA256 /v $Path
    if ($LASTEXITCODE -ne 0) { throw "Echec de signature : $Path (code $LASTEXITCODE)" }
    & $signtool verify /pa /v $Path | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Verification de signature echouee : $Path" }
    Write-Host "    signe + verifie OK"
}

if ($signtool) {
    $toSign = @("faraday.exe", "faraday_helper.exe")
    if (Test-Path (Join-Path $stage "faraday.dll")) { $toSign += "faraday.dll" }
    foreach ($exe in $toSign) {
        $p = Join-Path $stage $exe
        if (Test-Path $p) { Invoke-FaradaySign $p } else { Write-Warning "absent, non signe : $exe" }
    }
}

# --- 4) Zip portable ----------------------------------------------------------
$zip = Join-Path $distOut "Faraday-$version-x64-portable.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Write-Host "==> Creation du zip portable : $zip"
Compress-Archive -Path "$stage\*" -DestinationPath $zip -CompressionLevel Optimal
$zipMb = [math]::Round((Get-Item $zip).Length / 1MB, 1)
Write-Host "    (zip : $zipMb Mo)"

# --- 5) Installateur Inno Setup (si demande) + signature ----------------------
if ($Inno) {
    $iscc = (Get-Command ISCC.exe -ErrorAction SilentlyContinue).Source
    if (-not $iscc) {
        foreach ($c in @("$PSScriptRoot\..\tools\InnoSetup\ISCC.exe", "C:\Program Files (x86)\Inno Setup 6\ISCC.exe")) {
            if (Test-Path $c) { $iscc = $c; break }
        }
    }
    if (-not $iscc) {
        Write-Warning "ISCC.exe (Inno Setup) introuvable - installe-le puis relance avec -Inno."
    } else {
        Write-Host "==> Compilation installateur Inno ..."
        & $iscc "$PSScriptRoot\faraday.iss"
        if ($LASTEXITCODE -ne 0) { throw "Inno Setup a echoue (code $LASTEXITCODE)" }
        $setup = Join-Path $distOut "Faraday-Setup-$version-x64.exe"
        if ($signtool -and (Test-Path $setup)) { Invoke-FaradaySign $setup }
    }
}

# --- 6) Sommes de controle (integrite des archives non signables) -------------
$sums = Join-Path $distOut "SHA256SUMS.txt"
$lines = @()
foreach ($f in (Get-ChildItem $distOut -File | Where-Object { $_.Name -ne "SHA256SUMS.txt" })) {
    $lines += ("{0}  {1}" -f (Get-FileHash $f.FullName -Algorithm SHA256).Hash, $f.Name)
}
if ($lines.Count -gt 0) {
    $lines | Set-Content -Path $sums -Encoding ASCII
    Write-Host "==> Sommes SHA-256 ecrites : $sums"
}

Write-Host ""
Write-Host "TERMINE. Sorties :"
Write-Host "  - dossier app  : $stage"
Write-Host "  - zip portable : $zip"
if ($Inno) { Write-Host "  - installateur : $distOut\Faraday-Setup-$version-x64.exe" }
Write-Host "  - sommes       : $sums"
if ($signtool) { Write-Host "  - signature    : OUI (certificat $CertPath)" } else { Write-Host "  - signature    : non (aucun certificat fourni)" }
