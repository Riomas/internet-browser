# Faraday - test de bout en bout de l'INSTALLEUR dans un Windows Sandbox vierge.
#
#   Phase A : installation silencieuse (/VERYSILENT) -> le certificat doit etre
#             approuve par defaut et l'application installee doit demarrer.
#   Phase B : desinstallation silencieuse -> le certificat doit etre retire.
#   Phase C : installation silencieuse avec /NOCERT=1 -> pas d'approbation,
#             donc l'application ne doit PAS demarrer (comportement documente).
#
# Journaux recopies dans C:\Diag\out (visibles depuis l'hote). ASCII pur.

$ErrorActionPreference = 'Continue'
$Out = 'C:\Diag\out'
$Dist = 'C:\Dist'
$Setup = Join-Path $Dist 'Faraday-Setup-0.1.1-x64.exe'
$Helper = Join-Path $Dist 'Faraday\faraday-certificat.ps1'
# Dossier d'installation reel (voir [Setup] DefaultDirName dans packaging\faraday.iss).
$AppDir = Join-Path $env:LOCALAPPDATA 'Faraday'
$Exe = Join-Path $AppDir 'faraday.exe'

New-Item -ItemType Directory -Force -Path $Out | Out-Null

function Write-Log([string]$m) {
    Add-Content -Path (Join-Path $Out 'install-e2e.log') -Value ('[{0}] {1}' -f (Get-Date -Format 'HH:mm:ss'), $m)
}

function Test-Certificat([string]$libelle) {
    & powershell -NoProfile -ExecutionPolicy Bypass -File $Helper -Status | Out-Null
    $approuve = ($LASTEXITCODE -eq 0)
    Write-Log ("[{0}] certificat approuve = {1} (code {2})" -f $libelle, $approuve, $LASTEXITCODE)
    return $approuve
}

function Invoke-Silencieux([string]$titre, [string]$Fichier, [string]$Arguments, [int]$maxSec) {
    if (-not (Test-Path $Fichier)) {
        Write-Log ("[{0}] ABSENT : {1}" -f $titre, $Fichier)
        return $false
    }
    Write-Log ("[{0}] lancement : {1} {2}" -f $titre, (Split-Path $Fichier -Leaf), $Arguments)
    $p = Start-Process -FilePath $Fichier -ArgumentList $Arguments -PassThru
    $fini = $p.WaitForExit($maxSec * 1000)
    Write-Log ("[{0}] termine={1} code={2}" -f $titre, $fini, $p.ExitCode)
    return $fini
}

function Test-Lancement([string]$phase, [int]$secondes) {
    Remove-Item (Join-Path $AppDir 'debug.log') -Force -ErrorAction SilentlyContinue
    if (-not (Test-Path $Exe)) {
        Write-Log ("[{0}] faraday.exe absent ({1})" -f $phase, $Exe)
        return $false
    }
    $p = Start-Process -FilePath $Exe -WorkingDirectory $AppDir -PassThru
    $vivant = -not $p.WaitForExit($secondes * 1000)
    $nb = (Get-Process -Name faraday -ErrorAction SilentlyContinue | Measure-Object).Count
    Write-Log ("[{0}] processus vivant apres {1}s : {2} | processus faraday : {3}" -f $phase, $secondes, $vivant, $nb)
    if (-not $vivant) { Write-Log ("[{0}] code de sortie : {1}" -f $phase, $p.ExitCode) }
    Get-Process -Name faraday, faraday_helper -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 3
    $dbg = Join-Path $AppDir 'debug.log'
    if (Test-Path $dbg) {
        Copy-Item $dbg (Join-Path $Out ("{0}-debug.log" -f $phase)) -Force
        Write-Log ("[{0}] debug.log present : le bootstrap a refuse de demarrer" -f $phase)
    }
    return $vivant
}

# ---------------------------------------------------------------- Phase A
Write-Log '=== Phase A : installation silencieuse (approbation par defaut) ==='
$a0 = Test-Certificat 'A-avant-installation'
Invoke-Silencieux 'A-installation' $Setup ('/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /LOG="' + (Join-Path $Out 'A-installation-setup.log') + '"') 900
Write-Log ("[A] faraday.exe installe = {0}" -f (Test-Path $Exe))
$a1 = Test-Certificat 'A-apres-installation'
$a2 = Test-Lancement 'A-lancement' 25

# ---------------------------------------------------------------- Phase B
Write-Log '=== Phase B : desinstallation silencieuse ==='
$unins = Join-Path $AppDir 'unins000.exe'
$b0 = Invoke-Silencieux 'B-desinstallation' $unins '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART' 600
Start-Sleep -Seconds 5
Write-Log ("[B] dossier d'installation encore present = {0}" -f (Test-Path $AppDir))
$b1 = -not (Test-Certificat 'B-apres-desinstallation')

# ---------------------------------------------------------------- Phase C
Write-Log '=== Phase C : installation silencieuse avec /NOCERT=1 ==='
Invoke-Silencieux 'C-installation' $Setup ('/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /NOCERT=1 /LOG="' + (Join-Path $Out 'C-installation-setup.log') + '"') 900
$c1 = Test-Certificat 'C-apres-installation'
$c2 = Test-Lancement 'C-lancement' 15

Write-Log ("=== RESULTAT : A sans cert avant={0} / installe+demarre={1} | B certificat retire={2} | C non approuve={3} et ne demarre pas={4} ===" -f `
    (-not $a0), ($a1 -and $a2), $b1, (-not $c1), (-not $c2))
Start-Sleep -Seconds 2
shutdown /s /t 5
