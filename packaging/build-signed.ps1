#Requires -Version 5.1
<#
  Faraday - build signe (raccourci).

  Utilise par defaut le certificat de TEST cree par make-testcert.ps1
  (certs\faraday-test.pfx). Pour un vrai certificat de distribution,
  passer -CertPath / -CertPass.

  Usage :
    .\packaging\build-signed.ps1                              # zip + installateur signes (cert de test)
    .\packaging\build-signed.ps1 -NoInstaller                 # sans installateur Inno
    .\packaging\build-signed.ps1 -CertPath "C:\prod.pfx" -CertPass "***"
#>
[CmdletBinding()]
param(
    [string]$CertPath = "",
    [string]$CertPass = "faraday-test",
    [switch]$NoInstaller
)

$ErrorActionPreference = "Stop"

if (-not $CertPath) {
    $CertPath = Join-Path (Split-Path -Parent $PSScriptRoot) "certs\faraday-test.pfx"
}
if (-not (Test-Path $CertPath)) {
    Write-Error "Certificat introuvable : $CertPath`nCree-le avec : .\packaging\make-testcert.ps1"
}

$buildArgs = @("-CertPath", $CertPath, "-CertPass", $CertPass)
if (-not $NoInstaller) { $buildArgs += "-Inno" }

Write-Host "==> Build signe (certificat : $CertPath)"
& (Join-Path $PSScriptRoot "build-release.ps1") @buildArgs

Write-Host ""
Write-Host "Verification des signatures :"
foreach ($f in @("dist\Faraday\faraday.exe", "dist\Faraday\faraday_helper.exe", "dist\Faraday\faraday.dll", "dist\Faraday-Setup-0.1.0-x64.exe")) {
    $p = Join-Path (Split-Path -Parent $PSScriptRoot) $f
    if (Test-Path $p) {
        $s = Get-AuthenticodeSignature $p
        Write-Host ("  {0,-32} {1}" -f (Split-Path $p -Leaf), $s.Status)
    }
}
