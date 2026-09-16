# Faraday - approbation du certificat de signature (auto-signe).
#
# Cette version de Faraday est signee avec un certificat propre au projet, non
# delivre par une autorite de certification. Pour que Windows affiche le bon
# editeur - et pour que l'application signee puisse demarrer (le bootstrap de
# Chromium verifie la signature) - ce certificat doit etre approuve sur la
# machine, pour l'utilisateur courant uniquement (aucun droit administrateur).
#
# Usage :
#   powershell -ExecutionPolicy Bypass -File faraday-certificat.ps1            (installe)
#   powershell -ExecutionPolicy Bypass -File faraday-certificat.ps1 -Status    (verifie)
#   powershell -ExecutionPolicy Bypass -File faraday-certificat.ps1 -Remove    (retire)
#
# Aucun acces reseau, aucune modification ailleurs que dans le magasin de
# certificats de l'utilisateur courant.
#
# Fichier volontairement 100 % ASCII (PowerShell 5.1 lit les .ps1 en ANSI).

param(
    [switch]$Install,
    [switch]$Remove,
    [switch]$Status
)

$ErrorActionPreference = 'Stop'

$cerPath = Join-Path $PSScriptRoot 'faraday-certificat.cer'
if (-not (Test-Path $cerPath)) {
    Write-Host "[ERREUR] Certificat introuvable a cote du script : $cerPath"
    exit 2
}

try {
    $cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2($cerPath)
} catch {
    Write-Host "[ERREUR] Certificat illisible : $($_.Exception.Message)"
    exit 2
}

$sujet = $cert.Subject
$empreinte = $cert.Thumbprint
$store = New-Object System.Security.Cryptography.X509Certificates.X509Store('Root', 'CurrentUser')

function Test-Present {
    param($Store, $Thumbprint)
    $Store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadOnly)
    try {
        return [bool]($Store.Certificates | Where-Object { $_.Thumbprint -eq $Thumbprint })
    } finally {
        $Store.Close()
    }
}

if ($Status) {
    if (Test-Present -Store $store -Thumbprint $empreinte) {
        Write-Host "Certificat Faraday approuve ($empreinte)."
        exit 0
    }
    Write-Host "Certificat Faraday NON approuve sur ce compte."
    exit 1
}

if ($Remove) {
    $store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
    try {
        $store.Remove($cert)
    } catch {
        # Absent du magasin : rien a faire.
    } finally {
        $store.Close()
    }
    Write-Host "Certificat Faraday retire du magasin de l'utilisateur courant."
    exit 0
}

# Par defaut (aucune option, ou -Install) : on approuve le certificat.
$store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
try {
    $store.Add($cert)
} finally {
    $store.Close()
}

if (Test-Present -Store $store -Thumbprint $empreinte) {
    Write-Host "Certificat Faraday approuve pour ce compte utilisateur."
    Write-Host "  sujet    : $sujet"
    Write-Host "  empreinte: $empreinte"
    Write-Host "  portee   : utilisateur courant uniquement (retirable via -Remove)"
    exit 0
}

Write-Host "[ERREUR] L'ajout du certificat a echoue."
exit 1
