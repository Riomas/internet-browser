# Faraday - test de lancement de la variante sandbox dans un Windows Sandbox.
# Verifie que faraday.exe (bootstrap CEF) demarre et reste actif, puis recopie
# les journaux dans C:\Diag\out (visibles depuis l'hote). ASCII pur.

$ErrorActionPreference = 'Continue'
$App = 'C:\FaradayApp'
$Out = 'C:\Diag\out'

New-Item -ItemType Directory -Force -Path $Out | Out-Null
function Write-Log([string]$m) {
    Add-Content -Path (Join-Path $Out 'launch.log') -Value ('[{0}] {1}' -f (Get-Date -Format 'HH:mm:ss'), $m)
}

Write-Log 'test de lancement : demarrage de faraday.exe'
$p = Start-Process -FilePath (Join-Path $App 'faraday.exe') -WorkingDirectory $App -PassThru
$exitOnTime = $p.WaitForExit(25000)
$alive = -not $p.HasExited
Write-Log ('arret spontane : ' + $exitOnTime + ' / processus encore vivant : ' + $alive)
if (-not $alive) { Write-Log ('code de sortie : ' + $p.ExitCode) }
Write-Log ('processus faraday : ' + (Get-Process -Name faraday -ErrorAction SilentlyContinue | Measure-Object).Count)
Write-Log ('processus faraday_helper : ' + (Get-Process -Name faraday_helper -ErrorAction SilentlyContinue | Measure-Object).Count)

Get-Process -Name faraday, faraday_helper -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

foreach ($f in @('faraday-startup.log', 'debug.log')) {
    $src = Join-Path $App $f
    if (Test-Path $src) { Copy-Item $src (Join-Path $Out $f) -Force }
}
$prof = Join-Path $env:APPDATA 'Faraday'
if (Test-Path $prof) {
    foreach ($f in @('profiles.json', 'profiles\default\session.json', 'profiles\default\privacy.toml', 'profiles\default\startup.log')) {
        $src = Join-Path $prof $f
        if (Test-Path $src) { Copy-Item $src (Join-Path $Out ('profil-' + ($f -replace '\\', '_'))) -Force }
    }
}

Write-Log 'fin du test'
Start-Sleep -Seconds 2
shutdown /s /t 5
