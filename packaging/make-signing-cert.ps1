#Requires -Version 5.1
<#
  Faraday - certificat de signature DEFINITIF (auto-signe).

  Cree le certificat qui signera les distributions Faraday, l'exporte en .pfx
  (cle privee, a garder secrete) et en .cer (cle publique, jointe au lot).

  Sujet retenu : CN=Faraday, O=Faraday Project
    - le CN est le nom affiche par Windows comme EDITEUR ("Publie par : Faraday") ;
    - la racine de confiance etant identifiee par sa CLE, ce nom ne doit plus
      changer : sinon chaque utilisateur devrait re-approuver le certificat.

  Le certificat est auto-signe : c'est l'utilisateur qui l'approuve, sur son compte
  seulement, via l'utilitaire fourni dans le lot (packaging/installer/faraday-certificat.ps1)
  ou par la page dediee de l'installateur. Voir docs/CERTIFICAT_AUTOSIGNE.md.

  Usage :
    .\packaging\make-signing-cert.ps1                 # cree le certificat + les exports
    .\packaging\make-signing-cert.ps1 -Trust          # ... et l'approuve sur CETTE machine
    .\packaging\make-signing-cert.ps1 -Remove         # tout supprimer (creation refaite apres)
    .\packaging\make-signing-cert.ps1 -Force          # recreer malgre un certificat existant

  ATTENTION : ne jamais committer le .pfx ni son mot de passe ("certs/" est ignore par git).
  Fichier volontairement 100 % ASCII (PowerShell 5.1 lit les .ps1 en ANSI).
#>
[CmdletBinding()]
param(
    [string]$Subject = "CN=Faraday, O=Faraday Project",
    [string]$FriendlyName = "Faraday (certificat de signature)",
    [string]$PfxPath = "",
    [string]$CerPath = "",
    [int]$Years = 10,
    [string]$Passphrase = "",
    [switch]$Trust,
    [switch]$Remove,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
if (-not $PfxPath) { $PfxPath = Join-Path $root "certs\faraday.pfx" }
if (-not $CerPath) { $CerPath = Join-Path $root "certs\faraday.cer" }
$PassFile = Join-Path (Split-Path $PfxPath) "faraday-password.txt"

function Get-FaradayCerts {
    Get-ChildItem "Cert:\CurrentUser\My" -CodeSigningCert -ErrorAction SilentlyContinue |
        Where-Object { $_.FriendlyName -eq $FriendlyName }
}

function Remove-Fichiers {
    foreach ($f in @($PfxPath, $CerPath, $PassFile)) {
        if ($f -and (Test-Path $f)) { Remove-Item $f -Force }
    }
}

# ------------------------------------------------------------------ suppression
if ($Remove) {
    $certs = @(Get-FaradayCerts)
    if ($certs.Count -eq 0) {
        Write-Host "==> Aucun certificat de signature Faraday trouve dans le magasin utilisateur."
    }
    foreach ($c in $certs) {
        Write-Host "    suppression : $($c.Thumbprint)"
        foreach ($store in @("Cert:\CurrentUser\Root", "Cert:\CurrentUser\TrustedPublisher")) {
            Get-ChildItem $store -ErrorAction SilentlyContinue |
                Where-Object { $_.Thumbprint -eq $c.Thumbprint } |
                Remove-Item -Force -ErrorAction SilentlyContinue
        }
        Remove-Item -Path ("Cert:\CurrentUser\My\" + $c.Thumbprint) -Force
    }
    Remove-Fichiers
    Write-Host "==> Nettoyage termine (fichiers .pfx / .cer / mot de passe supprimes)."
    return
}

# --------------------------------------------- deja present ? (eviter 2 racines)
$existants = @(Get-FaradayCerts)
if ($existants.Count -gt 0 -and -not $Force) {
    Write-Host "==> Un certificat de signature Faraday existe deja :"
    foreach ($c in $existants) {
        Write-Host "    sujet     : $($c.Subject)"
        Write-Host "    empreinte : $($c.Thumbprint)"
        Write-Host "    expire le : $($c.NotAfter.ToString('yyyy-MM-dd'))"
    }
    Write-Host ""
    Write-Host "Rien a faire. Creer un SECOND certificat avec le meme nom obligerait chaque"
    Write-Host "utilisateur a approuver deux racines : utiliser -Force pour le remplacer"
    Write-Host "(ou -Remove pour tout supprimer, puis relancer ce script)."
    if (Test-Path $PfxPath) { return }
    Write-Host ""
    Write-Host "Le fichier .pfx est absent : reexport du certificat existant."
    $cert = $existants[0]
    if (Test-Path $CerPath) {
        Write-Host "    .cer deja present : $CerPath"
    } else {
        Export-Certificate -Cert $cert -FilePath $CerPath | Out-Null
        Write-Host "    .cer exporte : $CerPath"
    }
    if (-not $Passphrase) {
        Write-Host "[ATTENTION] Mot de passe inconnu : utiliser -Passphrase pour reexporter le .pfx."
        return
    }
    $secure = ConvertTo-SecureString -String $Passphrase -Force -AsPlainText
    Export-PfxCertificate -Cert $cert -FilePath $PfxPath -Password $secure | Out-Null
    Set-Content -Path $PassFile -Value $Passphrase -NoNewline
    Write-Host "    .pfx reexporte : $PfxPath"
    return
}

# -------------------------------------------------------------------- creation
if ($Force) {
    Write-Host "==> Remplacement du certificat existant"
    foreach ($c in $existants) {
        foreach ($store in @("Cert:\CurrentUser\Root", "Cert:\CurrentUser\TrustedPublisher")) {
            Get-ChildItem $store -ErrorAction SilentlyContinue |
                Where-Object { $_.Thumbprint -eq $c.Thumbprint } |
                Remove-Item -Force -ErrorAction SilentlyContinue
        }
        Remove-Item -Path ("Cert:\CurrentUser\My\" + $c.Thumbprint) -Force
    }
    Remove-Fichiers
}

New-Item -ItemType Directory -Force -Path (Split-Path $PfxPath) | Out-Null

Write-Host "==> Creation du certificat de signature (magasin utilisateur, aucun droit administrateur)"
Write-Host "    sujet     : $Subject"
Write-Host "    validite  : $Years ans"
$cert = New-SelfSignedCertificate `
    -Type CodeSigningCert `
    -Subject $Subject `
    -FriendlyName $FriendlyName `
    -CertStoreLocation "Cert:\CurrentUser\My" `
    -KeyUsage DigitalSignature `
    -KeyAlgorithm RSA -KeyLength 4096 `
    -HashAlgorithm SHA256 `
    -NotBefore (Get-Date).AddDays(-1) `
    -NotAfter (Get-Date).AddYears($Years)

if (-not $Passphrase) {
    $alphabet = "abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789"
    $Passphrase = -join (1..24 | ForEach-Object { $alphabet[(Get-Random -Maximum $alphabet.Length)] })
    $auto = $true
} else {
    $auto = $false
}

$secure = ConvertTo-SecureString -String $Passphrase -Force -AsPlainText
Export-PfxCertificate -Cert $cert -FilePath $PfxPath -Password $secure | Out-Null
Export-Certificate -Cert $cert -FilePath $CerPath | Out-Null
if ($auto) { Set-Content -Path $PassFile -Value $Passphrase -NoNewline }

Write-Host ""
Write-Host "==> Certificat cree"
Write-Host "    sujet      : $($cert.Subject)"
Write-Host "    empreinte  : $($cert.Thumbprint)"
Write-Host "    valide du  : $($cert.NotBefore.ToString('yyyy-MM-dd'))"
Write-Host "    valide au  : $($cert.NotAfter.ToString('yyyy-MM-dd'))"
Write-Host "    .pfx (cle privee, SECRET) : $PfxPath"
Write-Host "    .cer (cle publique, a distribuer) : $CerPath"
if ($auto) {
    Write-Host "    mot de passe genere : $PassFile"
    Write-Host "      -> a recopier dans un gestionnaire de mots de passe, puis supprimer ce fichier."
} else {
    Write-Host "    mot de passe : celui fourni en parametre (non enregistre)."
}

if ($Trust) {
    # Meme code que l'utilitaire du lot (packaging/installer/faraday-certificat.ps1) :
    # ajout direct au magasin, sans dialogue d'approbation interactif.
    $public = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2($CerPath)
    foreach ($emplacement in @('Root', 'TrustedPublisher')) {
        $store = New-Object System.Security.Cryptography.X509Certificates.X509Store($emplacement, 'CurrentUser')
        $store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
        try { $store.Add($public) } finally { $store.Close() }
    }
    Write-Host "==> Certificat approuve sur CETTE machine (magasins utilisateur : Root + TrustedPublisher)."
} else {
    Write-Host ""
    Write-Host "    Pour l'approuver sur cette machine (necessaire pour tester une version signee) :"
    Write-Host "      .\packaging\make-signing-cert.ps1 -Trust"
}

Write-Host ""
Write-Host "Etape suivante : construire et signer la distribution"
Write-Host "  .\packaging\build-release.ps1 -Sandbox -Inno -SignAppFiles ``"
Write-Host "      -CertPath `"$PfxPath`" -CertPass `"<mot de passe>`""
