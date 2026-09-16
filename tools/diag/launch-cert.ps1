# Faraday - verification du parcours "certificat auto-signe" dans un Windows
# Sandbox VIERGE (aucun certificat approuve au depart) :
#   Phase A : lancement SANS approbation -> le bootstrap CEF doit refuser.
#   Phase B : approbation via faraday-certificat.ps1 -> l'application doit demarrer.
# Journaux recopies dans C:\Diag\out (visibles depuis l'hote). ASCII pur.

$ErrorActionPreference = 'Continue'
$App = 'C:\FaradayApp'
$Out = 'C:\Diag\out'
$Exe = Join-Path $App 'faraday.exe'
$Cert = Join-Path $App 'faraday-certificat.ps1'

New-Item -ItemType Directory -Force -Path $Out | Out-Null
function Write-Log([string]$m) {
    Add-Content -Path (Join-Path $Out 'cert-launch.log') -Value ('[{0}] {1}' -f (Get-Date -Format 'HH:mm:ss'), $m)
}

function Test-Lancement([string]$nom, [int]$secondes) {
    Remove-Item (Join-Path $App 'debug.log') -Force -ErrorAction SilentlyContinue
    Remove-Item (Join-Path $App 'faraday-startup.log') -Force -ErrorAction SilentlyContinue

    $p = Start-Process -FilePath $Exe -WorkingDirectory $App -PassThru
    $resteVivant = -not $p.WaitForExit($secondes * 1000)
    $nb = (Get-Process -Name faraday -ErrorAction SilentlyContinue | Measure-Object).Count
    Write-Log ("[{0}] processus vivant apres {1}s : {2} | processus faraday : {3}" -f $nom, $secondes, $resteVivant, $nb)
    if (-not $resteVivant) { Write-Log ("[{0}] code de sortie : {1}" -f $nom, $p.ExitCode) }

    Get-Process -Name faraday, faraday_helper -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2

    foreach ($f in @('debug.log', 'faraday-startup.log')) {
        $src = Join-Path $App $f
        if (Test-Path $src) { Copy-Item $src (Join-Path $Out ("{0}-{1}" -f $nom, $f)) -Force }
    }
    return $resteVivant
}

Write-Log '=== Phase A : sans approbation du certificat ==='
$a = Test-Lancement 'A-sans-approbation' 15

Write-Log '=== Phase B : approbation du certificat (utilitaire du lot) ==='
$sortie = & powershell -NoProfile -ExecutionPolicy Bypass -File $Cert -Install 2>&1
foreach ($l in $sortie) { Write-Log ("  cert : " + $l) }
& powershell -NoProfile -ExecutionPolicy Bypass -File $Cert -Status 2>&1 | ForEach-Object { Write-Log ("  statut : " + $_) }

$b = Test-Lancement 'B-avec-approbation' 25

Write-Log ("=== RESULTAT : sans approbation demarre={0} | avec approbation demarre={1} ===" -f $a, $b)
Start-Sleep -Seconds 2
shutdown /s /t 5
