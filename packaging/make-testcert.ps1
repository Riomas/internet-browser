#Requires -Version 5.1
<#
  Faraday - certificat de signature de TEST (auto-signe).

  But : valider tout le pipeline de signature SANS acheter de certificat.
  ATTENTION : un certificat auto-signe n'est PAS reconnu par Smart App Control /
  SmartScreen. Il ne sert qu'a prouver que l'outillage fonctionne.

  Usage :
    .\packaging\make-testcert.ps1                 # cree le cert + exporte le .pfx
    .\packaging\make-testcert.ps1 -Trust          # ... et l'ajoute aux magasins de
                                                  #     confiance UTILISATEUR (Root + TrustedPublisher)
    .\packaging\make-testcert.ps1 -Remove         # supprime le cert de test et le .pfx
#>
[CmdletBinding()]
param(
    [string]$OutPfx = "",
    [string]$Password = "faraday-test",
    [string]$Subject = "CN=Faraday TEST (auto-signe, non fiable pour distribution)",
    [string]$FriendlyName = "Faraday TEST code signing",
    [switch]$Trust,
    [switch]$Remove
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
if (-not $OutPfx) { $OutPfx = Join-Path $root "certs\faraday-test.pfx" }

function Get-TestCerts {
    Get-ChildItem "Cert:\CurrentUser\My" -CodeSigningCert -ErrorAction SilentlyContinue |
        Where-Object { $_.FriendlyName -eq $FriendlyName -or $_.Subject -like "*Faraday TEST*" }
}

if ($Remove) {
    $certs = @(Get-TestCerts)
    if ($certs.Count -eq 0) { Write-Host "Aucun certificat de test trouve."; return }
    foreach ($c in $certs) {
        Write-Host "Suppression du certificat : $($c.Thumbprint)"
        foreach ($store in @("Cert:\CurrentUser\Root", "Cert:\CurrentUser\TrustedPublisher")) {
            Get-ChildItem $store -ErrorAction SilentlyContinue |
                Where-Object { $_.Thumbprint -eq $c.Thumbprint } |
                Remove-Item -Force -ErrorAction SilentlyContinue
        }
        Remove-Item -Path ("Cert:\CurrentUser\My\" + $c.Thumbprint) -Force
    }
    if (Test-Path $OutPfx) { Remove-Item $OutPfx -Force; Write-Host "PFX supprime : $OutPfx" }
    $cer = [System.IO.Path]::ChangeExtension($OutPfx, ".cer")
    if (Test-Path $cer) { Remove-Item $cer -Force }
    Write-Host "Nettoyage termine."
    return
}

Write-Host "==> Creation du certificat de test (magasin utilisateur, aucun droit admin)"
$cert = New-SelfSignedCertificate `
    -Type CodeSigningCert `
    -Subject $Subject `
    -FriendlyName $FriendlyName `
    -CertStoreLocation "Cert:\CurrentUser\My" `
    -KeyUsage DigitalSignature `
    -KeyAlgorithm RSA -KeyLength 3072 `
    -NotAfter (Get-Date).AddYears(1)

New-Item -ItemType Directory -Force -Path (Split-Path $OutPfx) | Out-Null
$secure = ConvertTo-SecureString -String $Password -Force -AsPlainText
Export-PfxCertificate -Cert $cert -FilePath $OutPfx -Password $secure | Out-Null
Write-Host "    PFX exporte : $OutPfx"
Write-Host "    Mot de passe (test) : $Password"
Write-Host "    Empreinte (thumbprint) : $($cert.Thumbprint)"

if ($Trust) {
    $cer = [System.IO.Path]::ChangeExtension($OutPfx, ".cer")
    Export-Certificate -Cert $cert -FilePath $cer | Out-Null
    Import-Certificate -FilePath $cer -CertStoreLocation "Cert:\CurrentUser\Root" | Out-Null
    Import-Certificate -FilePath $cer -CertStoreLocation "Cert:\CurrentUser\TrustedPublisher" | Out-Null
    Write-Host "==> Certificat ajoute aux magasins de confiance UTILISATEUR (Root + TrustedPublisher)."
    Write-Host "    (pour retirer : .\packaging\make-testcert.ps1 -Remove)"
}

Write-Host ""
Write-Host "Etape suivante : signer la distribution"
Write-Host "  .\packaging\build-release.ps1 -Inno -CertPath `"$OutPfx`" -CertPass `"$Password`""
