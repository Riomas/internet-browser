#Requires -Version 5.1
<#
  Faraday - declinaison du logo detoure en icones de toutes tailles.

  Entree  : crates\faraday\resources\icons\logo-transparent.png
            (produit par packaging\make-logo-detoure.ps1)

  Sorties :
    - crates\faraday\resources\icons\faraday.ico        (16/24/32/48/64/128/256,
                                                         entrees DIB 32bpp)
    - crates\faraday\resources\icons\faraday-64.rgba    (icone de fenetre egui)
    - crates\faraday\resources\icons\png\faraday-*.png  (16 -> 1024 px)
    - docs\favicon.ico                                  (favicon du site)
    - tools\diag\out\icones-logo\planche.png            (controle visuel)
    - tools\diag\out\icones-logo\logo-carre-1024.png

  Methode :
    1. recadrage sur la boite du canal alpha + marge de 5 % -> toile carree ;
    2. reduction par paliers successifs (evite le flou d'une reduction directe
       d'un facteur eleve) ;
    3. accentuation legere (unsharp) pour les tailles <= 32 px : canaux couleur
       seulement, en espace premultiplie et bornes par l'alpha (accentuer le
       canal alpha creait des pixels de couleur parasite sur les petites
       tailles : constate et corrige) ;
    4. ecriture du .ico (DIB 32bpp : compatible Explorer, ressources Win32,
       Inno Setup) et des PNG.

  Usage : powershell -ExecutionPolicy Bypass -File .\packaging\make-icons.ps1
  ASCII pur (PS 5.1 lit les scripts sans BOM en ANSI).
#>
[CmdletBinding()]
param(
    [string]$Source = "",
    [string]$Diag = "",
    [int]$Marge = 5,
    [double]$Nettete = 0.45,
    [int]$SeuilNettete = 32
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$root = Split-Path -Parent $PSScriptRoot
if ($Source -eq "") { $Source = Join-Path $root "crates\faraday\resources\icons\logo-transparent.png" }
if ($Diag -eq "") { $Diag = Join-Path $root "tools\diag\out\icones-logo" }
if (-not (Test-Path $Source)) {
    throw "Logo detoure introuvable : $Source (lancer d'abord make-logo-detoure.ps1)"
}

$dossierIcones = Join-Path $root "crates\faraday\resources\icons"
$dossierPng = Join-Path $dossierIcones "png"
$fichierIco = Join-Path $dossierIcones "faraday.ico"
$fichierRgba = Join-Path $dossierIcones "faraday-64.rgba"
$fichierFavicon = Join-Path $root "docs\favicon.ico"
New-Item -ItemType Directory -Force -Path $dossierPng | Out-Null
New-Item -ItemType Directory -Force -Path $Diag | Out-Null

# --- Outil en C# : recadrage, reduction, accentuation, ecriture .ico ---------
if (-not ("FaradayIcones" -as [type])) {
    Add-Type -ReferencedAssemblies "System.Drawing" -TypeDefinition @'
using System;
using System.Collections;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.IO;
using System.Runtime.InteropServices;

public static class FaradayIcones
{
    public static Bitmap Charger(string chemin)
    {
        Bitmap fichier = new Bitmap(chemin);
        Bitmap copie = new Bitmap(fichier.Width, fichier.Height, PixelFormat.Format32bppPArgb);
        Graphics g = Graphics.FromImage(copie);
        g.CompositingMode = CompositingMode.SourceCopy;
        g.DrawImage(fichier, 0, 0, fichier.Width, fichier.Height);
        g.Dispose();
        fichier.Dispose();
        return copie;
    }

    // boite englobante du canal alpha : { gauche, haut, droite, bas }
    public static int[] Boite(Bitmap bmp, int seuil)
    {
        int w = bmp.Width;
        int h = bmp.Height;
        BitmapData d = bmp.LockBits(new Rectangle(0, 0, w, h), ImageLockMode.ReadOnly, PixelFormat.Format32bppPArgb);
        byte[] p = new byte[w * h * 4];
        Marshal.Copy(d.Scan0, p, 0, p.Length);
        bmp.UnlockBits(d);
        int g = w;
        int t = h;
        int dr = -1;
        int b = -1;
        for (int y = 0; y < h; y++)
        {
            for (int x = 0; x < w; x++)
            {
                if (p[(y * w + x) * 4 + 3] <= seuil) continue;
                if (x < g) g = x;
                if (x > dr) dr = x;
                if (y < t) t = y;
                if (y > b) b = y;
            }
        }
        return new int[] { g, t, dr, b };
    }

    // toile carree : logo centre, marge exprimee en pourcentage du cote
    public static Bitmap Carre(Bitmap src, int margePct, out int cote)
    {
        int[] bb = Boite(src, 8);
        int bw = bb[2] - bb[0] + 1;
        int bh = bb[3] - bb[1] + 1;
        cote = (int)Math.Ceiling(Math.Max(bw, bh) * (1.0 + 2.0 * (double)margePct / 100.0));
        int cx = (bb[0] + bb[2]) / 2;
        int cy = (bb[1] + bb[3]) / 2;
        Bitmap dst = new Bitmap(cote, cote, PixelFormat.Format32bppPArgb);
        Graphics g = Graphics.FromImage(dst);
        g.Clear(Color.Transparent);
        g.CompositingMode = CompositingMode.SourceOver;
        g.InterpolationMode = InterpolationMode.HighQualityBicubic;
        g.PixelOffsetMode = PixelOffsetMode.HighQuality;
        g.CompositingQuality = CompositingQuality.HighQuality;
        g.SmoothingMode = SmoothingMode.HighQuality;
        g.DrawImage(src, cote / 2 - cx, cote / 2 - cy, src.Width, src.Height);
        g.Dispose();
        return dst;
    }

    // reduction : paliers successifs (evite le flou d'une reduction directe
    // d'un facteur eleve), puis accentuation optionnelle
    public static Bitmap Reduire(Bitmap src, int taille, double nettete)
    {
        Bitmap courant = src;
        bool aLiberer = false;
        while (courant.Width > taille * 2)
        {
            int n = courant.Width / 2;
            Bitmap etape = new Bitmap(n, n, PixelFormat.Format32bppPArgb);
            Graphics ge = Graphics.FromImage(etape);
            ge.Clear(Color.Transparent);
            ge.CompositingMode = CompositingMode.SourceOver;
            ge.InterpolationMode = InterpolationMode.HighQualityBicubic;
            ge.PixelOffsetMode = PixelOffsetMode.HighQuality;
            ge.CompositingQuality = CompositingQuality.HighQuality;
            ge.DrawImage(courant, 0, 0, n, n);
            ge.Dispose();
            if (aLiberer) courant.Dispose();
            courant = etape;
            aLiberer = true;
        }

        Bitmap dst = new Bitmap(taille, taille, PixelFormat.Format32bppPArgb);
        Graphics g = Graphics.FromImage(dst);
        g.Clear(Color.Transparent);
        g.CompositingMode = CompositingMode.SourceOver;
        g.InterpolationMode = InterpolationMode.HighQualityBicubic;
        g.PixelOffsetMode = PixelOffsetMode.HighQuality;
        g.CompositingQuality = CompositingQuality.HighQuality;
        g.DrawImage(courant, 0, 0, taille, taille);
        g.Dispose();
        if (aLiberer) courant.Dispose();
        if (nettete > 0.0) Accentuer(dst, nettete);
        return dst;
    }

    // accentuation (unsharp) : canaux couleur uniquement, en espace premultiplie,
    // avec contrainte de validite (canal <= alpha). Accentuer le canal alpha
    // produirait des depassements et, apres demultiplication, des pixels de
    // couleur parasite sur les petites tailles (constate et corrige).
    static void Accentuer(Bitmap bmp, double force)
    {
        int w = bmp.Width;
        int h = bmp.Height;
        if (w < 3 || h < 3) return;
        BitmapData d = bmp.LockBits(new Rectangle(0, 0, w, h), ImageLockMode.ReadWrite, PixelFormat.Format32bppPArgb);
        byte[] p = new byte[w * h * 4];
        Marshal.Copy(d.Scan0, p, 0, p.Length);
        byte[] src = (byte[])p.Clone();
        for (int y = 1; y < h - 1; y++)
        {
            for (int x = 1; x < w - 1; x++)
            {
                int i = (y * w + x) * 4;
                int a = src[i + 3];
                for (int c = 0; c < 3; c++)
                {
                    double centre = src[i + c];
                    double voisins = src[i - 4 + c] + src[i + 4 + c] + src[i - w * 4 + c] + src[i + w * 4 + c];
                    double v = centre * (1.0 + 4.0 * force) - force * voisins;
                    if (v < 0.0) v = 0.0;
                    if (v > a) v = a;
                    p[i + c] = (byte)Math.Round(v);
                }
            }
        }
        Marshal.Copy(p, 0, d.Scan0, p.Length);
        bmp.UnlockBits(d);
    }

    // pixels non premultiplies (BGRA dans l'ordre memoire, comme GDI+) --------
    public static byte[] Argb(Bitmap bmp)
    {
        int w = bmp.Width;
        int h = bmp.Height;
        BitmapData d = bmp.LockBits(new Rectangle(0, 0, w, h), ImageLockMode.ReadOnly, PixelFormat.Format32bppPArgb);
        byte[] p = new byte[w * h * 4];
        Marshal.Copy(d.Scan0, p, 0, p.Length);
        bmp.UnlockBits(d);
        byte[] o = new byte[p.Length];
        for (int i = 0; i < p.Length; i += 4)
        {
            int a = p[i + 3];
            if (a < 8) continue;   // presque transparent : pas de couleur a demultiplier
            o[i] = (byte)Math.Min(255, (p[i] * 255 + a / 2) / a);
            o[i + 1] = (byte)Math.Min(255, (p[i + 1] * 255 + a / 2) / a);
            o[i + 2] = (byte)Math.Min(255, (p[i + 2] * 255 + a / 2) / a);
            o[i + 3] = (byte)a;
        }
        return o;
    }

    // RGBA8 non premultiplie, ligne 0 = haut (format attendu par egui) --------
    public static byte[] Rgba(Bitmap bmp)
    {
        int w = bmp.Width;
        int h = bmp.Height;
        byte[] a = Argb(bmp);
        byte[] o = new byte[a.Length];
        for (int i = 0; i < a.Length; i += 4)
        {
            o[i] = a[i + 2];
            o[i + 1] = a[i + 1];
            o[i + 2] = a[i];
            o[i + 3] = a[i + 3];
        }
        return o;
    }

    // image DIB 32bpp (BITMAPINFOHEADER + pixels bas en haut + masque AND) ----
    public static byte[] Dib(byte[] argb, int w, int h)
    {
        MemoryStream ms = new MemoryStream();
        BinaryWriter bw = new BinaryWriter(ms);
        bw.Write((uint)40);
        bw.Write((int)w);
        bw.Write((int)(h * 2));
        bw.Write((ushort)1);
        bw.Write((ushort)32);
        bw.Write((uint)0);
        bw.Write((uint)(w * h * 4));
        bw.Write((int)0);
        bw.Write((int)0);
        bw.Write((uint)0);
        bw.Write((uint)0);
        for (int y = h - 1; y >= 0; y--)
        {
            for (int x = 0; x < w; x++)
            {
                int i = (y * w + x) * 4;
                bw.Write(argb[i]);
                bw.Write(argb[i + 1]);
                bw.Write(argb[i + 2]);
                bw.Write(argb[i + 3]);
            }
        }
        int ligneMasque = ((w + 31) / 32) * 4;
        bw.Write(new byte[ligneMasque * h]);
        bw.Flush();
        byte[] data = ms.ToArray();
        bw.Dispose();
        ms.Dispose();
        return data;
    }

    // fichier .ico complet ---------------------------------------------------
    public static byte[] Ico(IDictionary bitmaps, int[] tailles)
    {
        List<byte[]> images = new List<byte[]>();
        for (int k = 0; k < tailles.Length; k++)
        {
            Bitmap b = (Bitmap)bitmaps[tailles[k]];
            images.Add(Dib(Argb(b), tailles[k], tailles[k]));
        }
        int offset = 6 + 16 * images.Count;
        MemoryStream ms = new MemoryStream();
        BinaryWriter bw = new BinaryWriter(ms);
        bw.Write((ushort)0);
        bw.Write((ushort)1);
        bw.Write((ushort)images.Count);
        for (int k = 0; k < images.Count; k++)
        {
            int t = tailles[k];
            int dim = t >= 256 ? 0 : t;
            bw.Write((byte)dim);
            bw.Write((byte)dim);
            bw.Write((byte)0);
            bw.Write((byte)0);
            bw.Write((ushort)1);
            bw.Write((ushort)32);
            bw.Write((uint)images[k].Length);
            bw.Write((uint)offset);
            offset += images[k].Length;
        }
        for (int k = 0; k < images.Count; k++) bw.Write(images[k]);
        bw.Flush();
        byte[] data = ms.ToArray();
        bw.Dispose();
        ms.Dispose();
        return data;
    }
}
'@
}

# --- Recadrage carre ---------------------------------------------------------
$sourceBmp = [FaradayIcones]::Charger($Source)
$cote = 0
$maitre = [FaradayIcones]::Carre($sourceBmp, $Marge, [ref]$cote)
$boite = [FaradayIcones]::Boite($sourceBmp, 8)
Write-Host ""
Write-Host "Faraday - icones"
Write-Host ("  source            : {0} ({1}x{2})" -f $Source, $sourceBmp.Width, $sourceBmp.Height)
Write-Host ("  boite du logo     : {0}x{1} (x {2}..{3}, y {4}..{5})" -f ($boite[2] - $boite[0] + 1), ($boite[3] - $boite[1] + 1), $boite[0], $boite[2], $boite[1], $boite[3])
Write-Host ("  toile carree      : {0}x{0} (marge {1} %)" -f $cote, $Marge)

# --- Declinaisons ------------------------------------------------------------
$taillesPng = @(16, 20, 24, 32, 40, 48, 64, 96, 128, 256, 512, 1024)
$taillesIco = @(16, 24, 32, 48, 64, 128, 256)
$taillesFavicon = @(16, 32, 48)

$bitmaps = @{}
foreach ($t in ($taillesPng + $taillesIco)) {
    if ($bitmaps.ContainsKey($t)) { continue }
    $force = if ($t -le $SeuilNettete) { $Nettete } else { 0.0 }
    $bitmaps[$t] = [FaradayIcones]::Reduire($maitre, $t, $force)
}

foreach ($t in $taillesPng) {
    $chemin = Join-Path $dossierPng ("faraday-{0}.png" -f $t)
    $bitmaps[$t].Save($chemin, [System.Drawing.Imaging.ImageFormat]::Png)
}
Write-Host ("  PNG               : {0} fichiers 16 -> 1024 dans {1}" -f $taillesPng.Count, $dossierPng)

[System.IO.File]::WriteAllBytes($fichierIco, [FaradayIcones]::Ico($bitmaps, $taillesIco))
Write-Host ("  faraday.ico       : {0} (entrees {1}, {2} octets)" -f $fichierIco, ($taillesIco -join '/'), (Get-Item $fichierIco).Length)

[System.IO.File]::WriteAllBytes($fichierFavicon, [FaradayIcones]::Ico($bitmaps, $taillesFavicon))
Write-Host ("  favicon.ico       : {0} (entrees {1}, {2} octets)" -f $fichierFavicon, ($taillesFavicon -join '/'), (Get-Item $fichierFavicon).Length)

[System.IO.File]::WriteAllBytes($fichierRgba, [FaradayIcones]::Rgba($bitmaps[64]))
Write-Host ("  faraday-64.rgba   : {0} ({1} octets)" -f $fichierRgba, (Get-Item $fichierRgba).Length)

$bitmaps[1024].Save((Join-Path $Diag "logo-carre-1024.png"), [System.Drawing.Imaging.ImageFormat]::Png)

# --- Planche de controle -----------------------------------------------------
$clair = [System.Drawing.Color]::FromArgb(255, 255, 255, 255)
$sombre = [System.Drawing.Color]::FromArgb(255, 30, 41, 59)
$magenta = [System.Drawing.Color]::FromArgb(255, 255, 0, 255)

function New-Panneau {
    param([System.Drawing.Graphics]$G, [System.Drawing.Bitmap]$Bmp, [int]$X, [int]$Y, [int]$Cote,
          [System.Drawing.Color]$Fond, [string]$Legende, [bool]$TailleNative)
    $c = if ($TailleNative) { $Cote } else { $Cote }
    $tile = New-Object System.Drawing.Bitmap $c, $c, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $gt = [System.Drawing.Graphics]::FromImage($tile)
    $gt.Clear($Fond)
    if ($TailleNative) {
        $gt.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
        $gt.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
    } else {
        $gt.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $gt.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    }
    $gt.DrawImage($Bmp, 0, 0, $c, $c)
    $gt.Dispose()
    $G.DrawImage($tile, $X, $Y)
    $tile.Dispose()
    if ($Legende -ne "") { $G.DrawString($Legende, $script:police, $script:pinceau, [float]$X, [float]($Y + $c + 3)) }
}

$largeur = 820
$hauteur = 940
$planche = New-Object System.Drawing.Bitmap $largeur, $hauteur
$g = [System.Drawing.Graphics]::FromImage($planche)
$g.Clear([System.Drawing.Color]::FromArgb(255, 245, 246, 248))
$script:police = New-Object System.Drawing.Font 'Segoe UI', 9
$script:policeGras = New-Object System.Drawing.Font 'Segoe UI', 9, ([System.Drawing.FontStyle]::Bold)
$script:pinceau = [System.Drawing.Brushes]::Black

# ligne 1 : apercus 256 px
$g.DrawString("apercu 256 px - fond clair", $script:policeGras, $script:pinceau, 20, 8)
New-Panneau -G $g -Bmp $bitmaps[256] -X 20 -Y 26 -Cote 260 -Fond $clair -Legende "" -TailleNative $false
$g.DrawString("apercu 256 px - fond sombre", $script:policeGras, $script:pinceau, 320, 8)
New-Panneau -G $g -Bmp $bitmaps[256] -X 320 -Y 26 -Cote 260 -Fond $sombre -Legende "" -TailleNative $false

# lignes 2 et 3 : tailles natives
function New-Bande {
    param([System.Drawing.Graphics]$G, [int]$Y, [int[]]$Tailles, [System.Drawing.Color]$Fond, [string]$Titre)
    $G.DrawString($Titre, $script:policeGras, $script:pinceau, 20, ($Y - 16))
    $x = 20
    foreach ($t in $Tailles) {
        $tile = New-Object System.Drawing.Bitmap ($t + 10), ($t + 10), ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $gt = [System.Drawing.Graphics]::FromImage($tile)
        $gt.Clear($Fond)
        $gt.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
        $gt.DrawImage($bitmaps[$t], 5, 5, $t, $t)
        $gt.Dispose()
        $G.DrawImage($tile, $x, ($Y + $maxi - $t))
        $tile.Dispose()
        $G.DrawString(("{0} px" -f $t), $script:police, $script:pinceau, $x, ($Y + $maxi + 4))
        $x += $t + 22
    }
}
$maxi = 128
$taillesNatives = @(128, 64, 48, 32, 24, 16)
New-Bande -G $g -Y 320 -Tailles $taillesNatives -Fond $clair -Titre "tailles natives - fond clair (Explorateur, bureau)"
New-Bande -G $g -Y 490 -Tailles $taillesNatives -Fond $sombre -Titre "tailles natives - fond sombre (barre des taches)"

# ligne 4 : zoom sur les pixels reels des petites tailles
$yz = 670
$g.DrawString("zoom - pixels reels, sans lissage", $script:policeGras, $script:pinceau, 20, ($yz - 16))
$x = 20
foreach ($z in @(@(24, 6, $sombre, '24 px x6 sombre'), @(32, 6, $sombre, '32 px x6 sombre'), @(16, 8, $clair, '16 px x8 clair'), @(16, 8, $magenta, '16 px x8 magenta'))) {
    $t = [int]$z[0]
    $f = [int]$z[1]
    $fond = [System.Drawing.Color]$z[2]
    $c = $t * $f
    New-Panneau -G $g -Bmp $bitmaps[$t] -X $x -Y $yz -Cote $c -Fond $fond -Legende ([string]$z[3]) -TailleNative $true
    $x += [Math]::Max($c + 24, 186)
}

$cheminPlanche = Join-Path $Diag "planche.png"
$planche.Save($cheminPlanche, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$planche.Dispose()

Write-Host ("  planche           : {0} ({1}x{2})" -f $cheminPlanche, $largeur, $hauteur)
Write-Host ""

foreach ($b in $bitmaps.Values) { $b.Dispose() }
$maitre.Dispose()
$sourceBmp.Dispose()
