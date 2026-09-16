# Faraday - verification des icones et metadonnees embarquees dans les binaires.
#
#   - compare l'icone associee a chaque binaire avec l'icone source (faraday.ico)
#     via une empreinte SHA-256 des pixels (32x32) ;
#   - affiche les metadonnees Windows (ProductName, FileDescription, version) ;
#   - produit une planche de comparaison PNG : tools\diag\out\icons\planche-icones.png
#
# Usage : powershell -ExecutionPolicy Bypass -File .\tools\diag\check-icons.ps1
# ASCII pur.

Add-Type -AssemblyName System.Drawing

$ErrorActionPreference = 'Continue'
$root = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$out = Join-Path $PSScriptRoot 'out\icons'
New-Item -ItemType Directory -Force -Path $out | Out-Null

$source = Join-Path $root 'crates\faraday\resources\icons\faraday.ico'
$dist = Join-Path $root 'dist'
$binaires = @(
    (Join-Path $dist 'Faraday\faraday.exe'),
    (Join-Path $dist 'Faraday\faraday_helper.exe'),
    (Join-Path $dist 'Faraday\faraday.dll'),
    (Join-Path $dist 'Faraday\chrome_elf.dll'),
    (Join-Path $dist 'Faraday-Setup-0.1.1-x64.exe')
)

function Get-IconePixels {
    param([string]$Chemin)
    try {
        $ico = [System.Drawing.Icon]::ExtractAssociatedIcon($Chemin)
        if (-not $ico) { return $null }
        $bmp = $ico.ToBitmap()
        $bmp32 = New-Object System.Drawing.Bitmap $bmp, 32, 32
        $ms = New-Object System.IO.MemoryStream
        $bmp32.Save($ms, [System.Drawing.Imaging.ImageFormat]::Bmp)
        $octets = $ms.ToArray()
        $ms.Dispose(); $bmp32.Dispose(); $bmp.Dispose(); $ico.Dispose()
        return $octets
    } catch {
        return $null
    }
}

function Get-Empreinte {
    param([byte[]]$Octets)
    if (-not $Octets) { return '(aucune icone)' }
    $sha = [System.Security.Cryptography.SHA256]::Create()
    return (($sha.ComputeHash($Octets) | ForEach-Object { $_.ToString('x2') }) -join '').Substring(0, 16)
}

# --- icone source : image 32x32 du .ico (meme taille que celle extraite des
#     binaires, pour que les empreintes soient comparables) --------------------
$srcEmpreinte = '(illisible)'
if (Test-Path $source) {
    $ico = New-Object System.Drawing.Icon($source, 32, 32)
    $bmp = New-Object System.Drawing.Bitmap $ico.ToBitmap(), 32, 32
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Bmp)
    $srcEmpreinte = Get-Empreinte $ms.ToArray()
    Write-Host ('Source  : faraday.ico  (image retenue : {0}x{1})  empreinte {2}' -f $ico.Width, $ico.Height, $srcEmpreinte)
    $ms.Dispose(); $bmp.Dispose(); $ico.Dispose()
} else {
    Write-Host "Source  : faraday.ico INTROUVABLE ($source)"
}

Write-Host ''
Write-Host ('{0,-30} {1,-18} {2,-10} {3}' -f 'Binaire', 'Empreinte icone', 'Icone OK', 'Metadonnees')
Write-Host ('{0,-30} {1,-18} {2,-10} {3}' -f ('-' * 30), ('-' * 18), ('-' * 10), ('-' * 40))

$resultats = @()
foreach ($b in $binaires) {
    $nom = Split-Path $b -Leaf
    if (-not (Test-Path $b)) {
        Write-Host ('{0,-30} {1}' -f $nom, 'FICHIER ABSENT')
        continue
    }
    $pixels = Get-IconePixels $b
    $emp = Get-Empreinte $pixels
    $ok = if ($emp -eq $srcEmpreinte) { 'OUI' } else { 'non' }
    $vi = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($b)
    $meta = ('Produit="{0}" Desc="{1}" Version="{2}"' -f $vi.ProductName, $vi.FileDescription, $vi.FileVersion)
    Write-Host ('{0,-30} {1,-18} {2,-10} {3}' -f $nom, $emp, $ok, $meta)
    $resultats += [pscustomobject]@{ Nom = $nom; Chemin = $b; Icone = $pixels }
}

# --- planche de comparaison --------------------------------------------------
$taille = 72
$largeur = 220 * $resultats.Count + 20
$planche = New-Object System.Drawing.Bitmap ($largeur, 150)
$g = [System.Drawing.Graphics]::FromImage($planche)
$g.Clear([System.Drawing.Color]::White)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
$police = New-Object System.Drawing.Font 'Segoe UI', 9
$pinceau = [System.Drawing.Brushes]::Black
$i = 0
foreach ($r in $resultats) {
    $x = 20 + ($i * 220)
    if ($r.Icone) {
        $ms = New-Object System.IO.MemoryStream (,$r.Icone)
        $bmp = [System.Drawing.Image]::FromStream($ms)
        $g.DrawImage($bmp, $x, 10, $taille, $taille)
        $bmp.Dispose(); $ms.Dispose()
    } else {
        $g.DrawString('(aucune icone)', $police, $pinceau, $x, 30)
    }
    $g.DrawString($r.Nom, $police, $pinceau, $x, 92)
    $g.DrawString(("32x32 : " + (Get-Empreinte $r.Icone)), $police, $pinceau, $x, 108)
    $i++
}
$cheminPlanche = Join-Path $out 'planche-icones.png'
$planche.Save($cheminPlanche, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $planche.Dispose()

Write-Host ''
Write-Host "Planche de comparaison : $cheminPlanche"
Write-Host ("Icone source          : $source")
