#Requires -Version 5.1
<#
  Faraday - detourage du logo officiel : le pourtour gris devient transparent.

  Entree  : crates\faraday\resources\icons\logo.png
            (logo officiel "bouclier bleu + F", fond gris uni)
  Sortie  : crates\faraday\resources\icons\logo-transparent.png

  Methode (traitement pixel, pas de baguette magique graphique) :
    1. couleur de fond auto-detectee = mediane de l'anneau de bord ;
    2. fond = remplissage par diffusion (4-connexite) depuis les bords, tolerance
       faible -> un fond ENFERME dans le logo reste opaque ;
    3. bande de transition (rayon limite autour du fond) = pixels d'anticrenelage
       melanges au fond ;
    4. fond + bande -> decomposition couleur/alpha ("color to alpha") :
       P = C x a + fond x (1 - a). La recomposition du resultat sur le gris
       redonne EXACTEMENT l'image d'origine (ecart max verifie et affiche).
       Le lisere clair (plus clair que le fond) est demele lui aussi : il reste
       blanc et opaque, sans halo gris sur fond sombre ;
    5. interieur du logo : alpha 255, aucune transparence parasite.

  Apercu de controle : tools\diag\out\icones-logo\apercu-detourage.png
  Usage : powershell -ExecutionPolicy Bypass -File .\packaging\make-logo-detoure.ps1
  ASCII pur (PS 5.1 lit les scripts sans BOM en ANSI).
#>
[CmdletBinding()]
param(
    [string]$Source = "",
    [string]$Out = "",
    [int]$Tolerance = 16,
    [int]$Rayon = 4,
    [int]$Pente = 32,
    [int]$AlphaMini = 12,
    [string]$Diag = ""
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$root = Split-Path -Parent $PSScriptRoot
if ($Source -eq "") { $Source = Join-Path $root "crates\faraday\resources\icons\logo.png" }
if ($Out -eq "") { $Out = Join-Path $root "crates\faraday\resources\icons\logo-transparent.png" }
if ($Diag -eq "") { $Diag = Join-Path $root "tools\diag\out\icones-logo" }

if (-not (Test-Path $Source)) { throw "Logo source introuvable : $Source" }
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Out) | Out-Null
New-Item -ItemType Directory -Force -Path $Diag | Out-Null

# --- Outil en C# : lecture, diffusion et decomposition couleur -> alpha -------
if (-not ("FaradayDetourage" -as [type])) {
    Add-Type -ReferencedAssemblies "System.Drawing" -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;

public static class FaradayDetourage
{
    // --- pixels -------------------------------------------------------------
    public static byte[] Lire(Bitmap bmp)
    {
        int w = bmp.Width;
        int h = bmp.Height;
        BitmapData d = bmp.LockBits(new Rectangle(0, 0, w, h), ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
        byte[] px = new byte[w * h * 4];
        Marshal.Copy(d.Scan0, px, 0, px.Length);
        bmp.UnlockBits(d);
        return px;
    }

    public static Bitmap Creer(byte[] px, int w, int h)
    {
        Bitmap bmp = new Bitmap(w, h, PixelFormat.Format32bppArgb);
        BitmapData d = bmp.LockBits(new Rectangle(0, 0, w, h), ImageLockMode.WriteOnly, PixelFormat.Format32bppArgb);
        Marshal.Copy(px, 0, d.Scan0, px.Length);
        bmp.UnlockBits(d);
        return bmp;
    }

    public static Bitmap Charger(string chemin)
    {
        Bitmap fichier = new Bitmap(chemin);
        Bitmap copie = new Bitmap(fichier.Width, fichier.Height, PixelFormat.Format32bppArgb);
        Graphics g = Graphics.FromImage(copie);
        g.CompositingMode = CompositingMode.SourceCopy;
        g.DrawImage(fichier, 0, 0, fichier.Width, fichier.Height);
        g.Dispose();
        fichier.Dispose();
        return copie;
    }

    // --- couleur de fond : mediane de l'anneau de bord ----------------------
    public static int[] Fond(byte[] px, int w, int h)
    {
        int bw = Math.Max(2, w / 200);
        List<int> lr = new List<int>();
        List<int> lg = new List<int>();
        List<int> lb = new List<int>();
        for (int y = 0; y < h; y++)
        {
            for (int x = 0; x < w; x++)
            {
                if (!(x < bw || y < bw || x >= w - bw || y >= h - bw)) continue;
                int i = (y * w + x) * 4;
                lr.Add(px[i + 2]);
                lg.Add(px[i + 1]);
                lb.Add(px[i]);
            }
        }
        int[] res = new int[3];
        res[0] = Median(lr);
        res[1] = Median(lg);
        res[2] = Median(lb);
        return res;
    }

    static int Median(List<int> v)
    {
        int[] t = v.ToArray();
        Array.Sort(t);
        return t[t.Length / 2];
    }

    static int Ecart(int r, int g, int b, int br, int bg, int bb)
    {
        int d = Math.Abs(r - br);
        int e = Math.Abs(g - bg);
        if (e > d) d = e;
        e = Math.Abs(b - bb);
        if (e > d) d = e;
        return d;
    }

    // --- fond (distance 0) + bande d'anticrenelage (distance 1..rayon) ------
    // Phase A : diffusion du fond, 4-connexite, tolerance faible, sans limite de
    //           distance (le fond entier y passe, un fond enferme non).
    // Phase B : extension limitee a `rayon` pixels vers les pixels de transition
    //           (melanges au fond) : tolerance elargie a chaque pas.
    public static byte[] Distances(byte[] px, int w, int h, int br, int bg, int bb, int tol, int rayon, int pente,
                                   out int pixelsFond, out int pixelsBande)
    {
        int n = w * h;
        byte[] dist = new byte[n];
        for (int i = 0; i < n; i++) dist[i] = 255;
        int[] pile = new int[n];
        int haut = 0;
        int x;
        int y;

        for (x = 0; x < w; x++)
        {
            Amorcer(px, w, h, dist, pile, ref haut, x, 0, br, bg, bb, tol);
            Amorcer(px, w, h, dist, pile, ref haut, x, h - 1, br, bg, bb, tol);
        }
        for (y = 0; y < h; y++)
        {
            Amorcer(px, w, h, dist, pile, ref haut, 0, y, br, bg, bb, tol);
            Amorcer(px, w, h, dist, pile, ref haut, w - 1, y, br, bg, bb, tol);
        }

        int[] dx = new int[] { 1, -1, 0, 0 };
        int[] dy = new int[] { 0, 0, 1, -1 };

        // Phase A : fond
        while (haut > 0)
        {
            int p = pile[--haut];
            x = p % w;
            y = p / w;
            for (int k = 0; k < 4; k++)
            {
                int nx = x + dx[k];
                int ny = y + dy[k];
                if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
                int q = ny * w + nx;
                if (dist[q] != 255) continue;
                int i = q * 4;
                if (Ecart(px[i + 2], px[i + 1], px[i], br, bg, bb) > tol) continue;
                dist[q] = 0;
                pile[haut] = q;
                haut++;
            }
        }

        // Phase B : bande de transition, en largeur d'abord (distance croissante)
        int[] file = new int[n];
        int tete = 0;
        int queue = 0;
        for (int i = 0; i < n; i++)
        {
            if (dist[i] == 0) { file[queue] = i; queue++; }
        }
        while (tete < queue)
        {
            int p = file[tete];
            tete++;
            int d = dist[p];
            if (d >= rayon) continue;
            x = p % w;
            y = p / w;
            for (int k = 0; k < 4; k++)
            {
                int nx = x + dx[k];
                int ny = y + dy[k];
                if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
                int q = ny * w + nx;
                if (dist[q] != 255) continue;
                int i = q * 4;
                int e = Ecart(px[i + 2], px[i + 1], px[i], br, bg, bb);
                if (e > tol + pente * (d + 1)) continue;
                dist[q] = (byte)(d + 1);
                file[queue] = q;
                queue++;
            }
        }

        pixelsFond = 0;
        pixelsBande = 0;
        for (int i = 0; i < n; i++)
        {
            if (dist[i] == 0) pixelsFond++;
            else if (dist[i] != 255) pixelsBande++;
        }
        return dist;
    }

    static void Amorcer(byte[] px, int w, int h, byte[] dist, int[] pile, ref int haut, int x, int y,
                        int br, int bg, int bb, int tol)
    {
        int q = y * w + x;
        if (dist[q] != 255) return;
        int i = q * 4;
        if (Ecart(px[i + 2], px[i + 1], px[i], br, bg, bb) > tol) return;
        dist[q] = 0;
        pile[haut] = q;
        haut++;
    }

    // --- color to alpha : P = C x a + fond x (1 - a) ------------------------
    public static byte[] Detourer(byte[] px, int w, int h, int br, int bg, int bb, byte[] dist, int alphaMini,
                                  out int demeles, out int transparents)
    {
        byte[] sortie = new byte[px.Length];
        Array.Copy(px, sortie, px.Length);
        demeles = 0;
        transparents = 0;
        int n = w * h;
        for (int i = 0; i < n; i++)
        {
            if (dist[i] == 255) continue;              // interieur du logo : conserve
            int j = i * 4;
            int b = px[j];
            int g = px[j + 1];
            int r = px[j + 2];
            double a = 0.0;
            a = Math.Max(a, Taux(r, br));
            a = Math.Max(a, Taux(g, bg));
            a = Math.Max(a, Taux(b, bb));
            int alpha = (int)Math.Round(a * 255.0);
            if (alpha <= alphaMini) alpha = 0;
            if (alpha == 0)
            {
                sortie[j] = 0;
                sortie[j + 1] = 0;
                sortie[j + 2] = 0;
                sortie[j + 3] = 0;
                transparents++;
            }
            else
            {
                double af = (double)alpha / 255.0;
                sortie[j] = Canal(b, bb, af);
                sortie[j + 1] = Canal(g, bg, af);
                sortie[j + 2] = Canal(r, br, af);
                sortie[j + 3] = (byte)alpha;
                demeles++;
            }
        }
        return sortie;
    }

    static double Taux(int c, int fond)
    {
        if (c >= fond)
        {
            if (fond >= 255) return 0.0;
            return (double)(c - fond) / (double)(255 - fond);
        }
        if (fond <= 0) return 0.0;
        return (double)(fond - c) / (double)fond;
    }

    static byte Canal(int c, int fond, double a)
    {
        double v = ((double)c - (double)fond * (1.0 - a)) / a;
        if (v < 0.0) v = 0.0;
        if (v > 255.0) v = 255.0;
        return (byte)Math.Round(v);
    }

    // --- controle : recomposition sur le fond vs image d'origine ------------
    public static int Verifier(byte[] px, byte[] res, byte[] dist, int br, int bg, int bb)
    {
        int max = 0;
        for (int i = 0; i < dist.Length; i++)
        {
            if (dist[i] == 255) continue;
            int j = i * 4;
            double a = res[j + 3] / 255.0;
            int[] fond = new int[] { bb, bg, br };
            int[] c = new int[] { res[j], res[j + 1], res[j + 2] };
            for (int k = 0; k < 3; k++)
            {
                int v = (int)Math.Round(c[k] * a + fond[k] * (1.0 - a));
                int e = Math.Abs(v - px[j + k]);
                if (e > max) max = e;
            }
        }
        return max;
    }

    // --- boite englobante du logo (alpha) -----------------------------------
    public static int[] Boite(byte[] px, int w, int h, int seuil)
    {
        int g = w;
        int t = h;
        int d = -1;
        int b = -1;
        for (int y = 0; y < h; y++)
        {
            for (int x = 0; x < w; x++)
            {
                if (px[(y * w + x) * 4 + 3] <= seuil) continue;
                if (x < g) g = x;
                if (x > d) d = x;
                if (y < t) t = y;
                if (y > b) b = y;
            }
        }
        return new int[] { g, t, d, b };
    }
}
'@
}

# --- Traitement --------------------------------------------------------------
$srcBmp = [FaradayDetourage]::Charger($Source)
$w = $srcBmp.Width
$h = $srcBmp.Height
$px = [FaradayDetourage]::Lire($srcBmp)

$fond = [FaradayDetourage]::Fond($px, $w, $h)
$fr = $fond[0]; $fg = $fond[1]; $fb = $fond[2]

Write-Host ""
Write-Host "Faraday - detourage du logo"
Write-Host ("  source           : {0} ({1}x{2})" -f $Source, $w, $h)
Write-Host ("  fond detecte     : R{0} V{1} B{2}" -f $fr, $fg, $fb)

$pixelsFond = 0
$pixelsBande = 0
$dist = [FaradayDetourage]::Distances($px, $w, $h, $fr, $fg, $fb, $Tolerance, $Rayon, $Pente,
                                       [ref]$pixelsFond, [ref]$pixelsBande)

$demeles = 0
$transparents = 0
$res = [FaradayDetourage]::Detourer($px, $w, $h, $fr, $fg, $fb, $dist, $AlphaMini,
                                    [ref]$demeles, [ref]$transparents)

$ecartMax = [FaradayDetourage]::Verifier($px, $res, $dist, $fr, $fg, $fb)
$boite = [FaradayDetourage]::Boite($res, $w, $h, 8)

Write-Host ("  tolerance / rayon: {0} / {1} px (pente {2})" -f $Tolerance, $Rayon, $Pente)
Write-Host ("  pixels fond      : {0}" -f $pixelsFond)
Write-Host ("  pixels transition: {0} (anti-aliasing demele)" -f $pixelsBande)
Write-Host ("  pixels opaques   : {0} + {1} partiels" -f $transparents, $demeles)
Write-Host ("  boite du logo    : x {0}..{1}  y {2}..{3}" -f $boite[0], $boite[2], $boite[1], $boite[3])
Write-Host ("  controle         : recomposition sur le fond, ecart max = {0}/255" -f $ecartMax)

$resBmp = [FaradayDetourage]::Creer($res, $w, $h)
$resBmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
Write-Host ("  ecrit            : {0} ({1} octets)" -f $Out, (Get-Item $Out).Length)

# --- Apercu de controle ------------------------------------------------------
$clair = [System.Drawing.Color]::FromArgb(255, 255, 255, 255)
$sombre = [System.Drawing.Color]::FromArgb(255, 30, 41, 59)
$magenta = [System.Drawing.Color]::FromArgb(255, 255, 0, 255)

function New-Panneau {
    param([System.Drawing.Bitmap]$Bmp, [System.Drawing.Graphics]$G, [int]$X, [int]$Y, [int]$Cote,
          [System.Drawing.Color]$Fond, [string]$Legende)
    $tmp = New-Object System.Drawing.Bitmap $Cote, $Cote, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $gt = [System.Drawing.Graphics]::FromImage($tmp)
    $gt.Clear($Fond)
    $gt.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $gt.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $gt.DrawImage($Bmp, 0, 0, $Cote, $Cote)
    $gt.Dispose()
    $G.DrawImage($tmp, $X, $Y)
    $tmp.Dispose()
    $G.DrawString($Legende, $script:police, $script:pinceau, [float]$X, [float]($Y + $Cote + 4))
}

function New-Zoom {
    param([System.Drawing.Bitmap]$Bmp, [System.Drawing.Graphics]$G, [int]$X, [int]$Y, [int]$X0, [int]$Y0,
          [int]$Cote, [int]$Facteur, [System.Drawing.Color]$Fond, [string]$Legende)
    $c = $Cote * $Facteur
    $tmp = New-Object System.Drawing.Bitmap $c, $c, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $gt = [System.Drawing.Graphics]::FromImage($tmp)
    $gt.Clear($Fond)
    $gt.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
    $gt.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
    $srcRect = New-Object System.Drawing.Rectangle $X0, $Y0, $Cote, $Cote
    $dstRect = New-Object System.Drawing.Rectangle 0, 0, $c, $c
    $gt.DrawImage($Bmp, $dstRect, $srcRect, [System.Drawing.GraphicsUnit]::Pixel)
    $gt.Dispose()
    $G.DrawImage($tmp, $X, $Y)
    $tmp.Dispose()
    $G.DrawString($Legende, $script:police, $script:pinceau, [float]$X, [float]($Y + $c + 4))
}

$cotePanneau = 320
$zoomCote = 160
$zoomFacteur = 4
$marge = 20
$largeur = (4 * $cotePanneau) + (5 * $marge)
$hauteur = $marge + $cotePanneau + 26 + $marge + ($zoomCote * $zoomFacteur) + 26 + $marge
$apercu = New-Object System.Drawing.Bitmap $largeur, $hauteur
$g = [System.Drawing.Graphics]::FromImage($apercu)
$g.Clear([System.Drawing.Color]::FromArgb(255, 245, 246, 248))
$script:police = New-Object System.Drawing.Font 'Segoe UI', 9
$script:pinceau = [System.Drawing.Brushes]::Black

New-Panneau -Bmp $srcBmp -G $g -X $marge -Y $marge -Cote $cotePanneau -Fond $clair -Legende "source (fond gris)"
$xp = $marge * 2 + $cotePanneau
New-Panneau -Bmp $resBmp -G $g -X $xp -Y $marge -Cote $cotePanneau -Fond $clair -Legende "resultat sur fond clair"
$xp = $marge * 3 + $cotePanneau * 2
New-Panneau -Bmp $resBmp -G $g -X $xp -Y $marge -Cote $cotePanneau -Fond $sombre -Legende "resultat sur fond sombre"
$xp = $marge * 4 + $cotePanneau * 3
New-Panneau -Bmp $resBmp -G $g -X $xp -Y $marge -Cote $cotePanneau -Fond $magenta -Legende "resultat sur magenta"

# zoom sur le bord haut-gauche du logo (controle des halos)
$x0 = [Math]::Max(0, $boite[0] - 14)
$y0 = [Math]::Max(0, $boite[1] - 14)
$yz = $marge + $cotePanneau + 26 + $marge
New-Zoom -Bmp $srcBmp -G $g -X $marge -Y $yz -X0 $x0 -Y0 $y0 -Cote $zoomCote -Facteur $zoomFacteur -Fond $clair -Legende ("zoom x{0} source (pixels reels)" -f $zoomFacteur)
$xzoom = $marge * 2 + ($zoomCote * $zoomFacteur)
New-Zoom -Bmp $resBmp -G $g -X $xzoom -Y $yz -X0 $x0 -Y0 $y0 -Cote $zoomCote -Facteur $zoomFacteur -Fond $magenta -Legende ("zoom x{0} resultat sur magenta (aucun halo gris)" -f $zoomFacteur)

$cheminApercu = Join-Path $Diag "apercu-detourage.png"
$apercu.Save($cheminApercu, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$apercu.Dispose()
$resBmp.Dispose()
$srcBmp.Dispose()

Write-Host ("  apercu           : {0}" -f $cheminApercu)
Write-Host ""
