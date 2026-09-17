# Faraday - test TEMOIN du controle d'application (Windows Sandbox / machine de test).
#
#   But : distinguer un probleme de NOS binaires d'une STRATEGIE de l'environnement.
#   Pour chaque fichier : tentative de lancement (Start-Process) + signature vue
#   depuis la machine de test. Journal : C:\Diag\out\test-blocage.log
#
#   Lecture du resultat :
#     - seuls les binaires signes Microsoft se lancent, les notres sont "BLOQUE"
#       (y compris une version precedente deja validee) -> l'environnement applique
#       une politique de controle d'application (Smart App Control / WDAC) :
#       test NON CONCLUANT, utiliser une VM ou un PC propre (docs/TEST_VM.md).
#     - nos binaires se lancent -> l'environnement est utilisable pour tester.
#
#   ASCII pur. Lance par test-blocage.wsb (LogonCommand).
$Out = 'C:\Diag\out'
$Dist = 'C:\Dist'
$log = Join-Path $Out 'test-blocage.log'
Set-Content -Path $log -Value ('=== test de lancement - ' + (Get-Date -Format 'yyyy-MM-dd HH:mm:ss') + ' ===') -Encoding ASCII

function Test-Lancement([string]$Libelle, [string]$Chemin) {
    if (-not (Test-Path $Chemin)) {
        Add-Content $log ("{0,-38} ABSENT" -f $Libelle)
        return
    }
    $sig = 'inconnue'
    try { $sig = (Get-AuthenticodeSignature $Chemin).Status } catch { }
    try {
        $p = Start-Process -FilePath $Chemin -PassThru -ErrorAction Stop
        Start-Sleep -Seconds 3
        $vivant = -not $p.HasExited
        Add-Content $log ("{0,-38} LANCE   pid={1} vivant={2} signature={3}" -f $Libelle, $p.Id, $vivant, $sig)
        Get-Process -Id $p.Id -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    } catch {
        Add-Content $log ("{0,-38} BLOQUE  signature={1} : {2}" -f $Libelle, $sig, $_.Exception.Message)
    }
}

# Temoin : binaire signe Microsoft (doit TOUJOURS se lancer).
Test-Lancement 'cmd.exe (Microsoft, temoin)' 'C:\Windows\System32\cmd.exe'

# Nos binaires : installateurs (nom versionne) + application.
$setups = @(Get-ChildItem $Dist -Filter 'Faraday-Setup-*-x64.exe' -ErrorAction SilentlyContinue | Sort-Object Name)
if ($setups.Count -eq 0) { Add-Content $log '                     (aucun Faraday-Setup-*-x64.exe dans C:\Dist)' }
foreach ($s in $setups) { Test-Lancement ($s.Name + ' (installateur)') $s.FullName }
Test-Lancement 'faraday.exe (application)' (Join-Path $Dist 'Faraday\faraday.exe')

Add-Content $log '=== fin ==='
Start-Sleep -Seconds 2
shutdown /s /t 5
