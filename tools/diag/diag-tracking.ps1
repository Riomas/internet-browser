# Faraday - banc de diagnostic : blocage reseau + exemption par site.
#
# Ce script s'execute DANS le Windows Sandbox (voir diag-tracking.wsb).
#   phases A/B : page de test locale (127.0.0.1 vs localhost) -> controle direct
#   phases C/D : vrai test EFF Cover Your Tracks (/kcarter?aat=1), sans puis
#                avec exemption du site -> reproduction du cas signale
# Les journaux sont recopies dans C:\Diag\out (donc visibles depuis l'hote).
#
# Fichier volontairement 100 % ASCII (PowerShell 5.1 lit les .ps1 en ANSI).

$ErrorActionPreference = 'Continue'
$App = 'C:\FaradayApp'
$Out = 'C:\Diag\out'

New-Item -ItemType Directory -Force -Path $Out | Out-Null
function Write-Log([string]$m) {
    Add-Content -Path (Join-Path $Out 'harness.log') -Value ('[{0}] {1}' -f (Get-Date -Format 'HH:mm:ss'), $m)
}
function Write-Text([string]$path, [string]$text) {
    # Ecriture sans BOM (serde_json n'accepte pas le BOM en tete de fichier).
    $enc = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($path, $text, $enc)
}

Write-Log 'demarrage du banc de test'

# --- Page de test servie en local (127.0.0.1:8765) -------------------------
$page = @'
<!doctype html>
<html><head><meta charset="utf-8"><title>Faraday diag</title></head>
<body>
<h1>Faraday diagnostic</h1>
<img src="https://trackersimulator.org/px.png" width="1" height="1">
<img src="https://sb.scorecardresearch.com/p?c1=2" width="1" height="1">
<script src="https://www.google-analytics.com/analytics.js"></script>
<iframe src="https://eviltracker.net/emb.html" width="40" height="20"></iframe>
</body></html>
'@

$server = Start-Job -ScriptBlock {
    param($html)
    $listener = New-Object System.Net.Sockets.TcpListener([System.Net.IPAddress]::Any, 8765)
    $listener.Start()
    $end = (Get-Date).AddMinutes(6)
    while ((Get-Date) -lt $end) {
        if ($listener.Pending()) {
            try {
                $client = $listener.AcceptTcpClient()
                $stream = $client.GetStream()
                $reader = New-Object System.IO.StreamReader($stream)
                $null = $reader.ReadLine()
                $bytes = [System.Text.Encoding]::UTF8.GetBytes($html)
                $head = "HTTP/1.1 200 OK`r`nContent-Type: text/html; charset=utf-8`r`nContent-Length: $($bytes.Length)`r`nCache-Control: no-store`r`nConnection: close`r`n`r`n"
                $hb = [System.Text.Encoding]::ASCII.GetBytes($head)
                $stream.Write($hb, 0, $hb.Length)
                $stream.Write($bytes, 0, $bytes.Length)
                $stream.Flush()
                $client.Close()
            } catch { }
        } else {
            Start-Sleep -Milliseconds 40
        }
    }
    $listener.Stop()
} -ArgumentList $page
Write-Log ("serveur de test demarre (job {0})" -f $server.Id)

# --- Connectivite Internet depuis le sandbox ------------------------------
try {
    $r = Invoke-WebRequest -Uri 'https://coveryourtracks.eff.org/' -UseBasicParsing -TimeoutSec 25
    Write-Log ("internet OK : HTTP {0} ({1} octets)" -f $r.StatusCode, $r.RawContentLength)
} catch {
    Write-Log ("internet INDISPONIBLE : " + $_.Exception.Message)
}

$profile = Join-Path $env:APPDATA 'Faraday\profiles\default'
New-Item -ItemType Directory -Force -Path $profile | Out-Null
$sessionFile = Join-Path $profile 'session.json'
$exceptFile = Join-Path $profile 'exceptions.json'
$diagFile = Join-Path $profile 'diag.log'

function Invoke-Phase([string]$name, [string]$url, [string]$exceptionsJson, [int]$seconds, [string]$pause = '') {
    Write-Log ("phase {0} : {1} (exceptions={2} pause={3})" -f $name, $url, $exceptionsJson, $pause)
    if ($pause -eq '1') { $env:FARADAY_PAUSE = '1' } else { Remove-Item Env:\FARADAY_PAUSE -ErrorAction SilentlyContinue }
    Write-Text $sessionFile ('{"active":0,"tabs":["' + $url + '"],"theme":0,"history":[]}')
    Write-Text $exceptFile $exceptionsJson
    Add-Content -Path $diagFile -Value ('--- PHASE ' + $name + ' : ' + $url + ' | exceptions=' + $exceptionsJson + ' | pause=' + $pause + ' ---')

    $p = Start-Process -FilePath (Join-Path $App 'faraday.exe') -WorkingDirectory $App -PassThru
    $null = $p.WaitForExit($seconds * 1000)
    Get-Process -Name faraday -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Get-Process -Name faraday_helper -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Write-Log 'phase terminee'
    Start-Sleep -Seconds 2
}

$env:FARADAY_DIAG = '1'   # active le journal diag.log de l'application
Invoke-Phase 'A-local-sans-exemption' 'http://127.0.0.1:8765/a.html' '[]' 18
Invoke-Phase 'B-local-site-exempte' 'http://localhost:8765/a.html' '["localhost"]' 18
Invoke-Phase 'C-eff-sans-exemption' 'https://coveryourtracks.eff.org/kcarter?aat=1' '[]' 35
Invoke-Phase 'D-eff-site-exempte' 'https://coveryourtracks.eff.org/kcarter?aat=1' '["coveryourtracks.eff.org"]' 35
Invoke-Phase 'E-eff-protection-suspendue' 'https://coveryourtracks.eff.org/kcarter?aat=1' '[]' 35 '1'

# --- Recuperation des journaux --------------------------------------------
Get-Process -Name faraday -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1
foreach ($f in @('diag.log', 'startup.log', 'session.json', 'exceptions.json')) {
    $src = Join-Path $profile $f
    if (Test-Path $src) { Copy-Item $src (Join-Path $Out $f) -Force }
}
$exeLog = Join-Path $App 'faraday-startup.log'
if (Test-Path $exeLog) { Copy-Item $exeLog (Join-Path $Out 'faraday-startup.log') -Force }

Stop-Job $server -ErrorAction SilentlyContinue
Remove-Job $server -Force -ErrorAction SilentlyContinue
Write-Log 'banc de test termine'
Start-Sleep -Seconds 2
shutdown /s /t 5
