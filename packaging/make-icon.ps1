#Requires -Version 5.1
<#
  Faraday — génération de l'icône applicative (faraday.ico).
  Bouclier vert « privacy » sur dalle sombre arrondie.
  Format .ico classique : entrées BMP/DIB 32bpp (compatible Explorer Win10/11,
  ressources Win32, Inno Setup).

  Sortie : crates\faraday\resources\icons\faraday.ico
  Usage   : powershell -ExecutionPolicy Bypass -File .\packaging\make-icon.ps1
#>
Add-Type -AssemblyName System.Drawing

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$outDir = Join-Path $root "crates\faraday\resources\icons"
$outFile = Join-Path $outDir "faraday.ico"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

# --- Dessine une taille (bitmap ARGB) ---------------------------------------
function New-ShieldBitmap {
    param([int]$Size)
    $bmp = New-Object System.Drawing.Bitmap $Size, $Size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.Clear([System.Drawing.Color]::Transparent)

    $s  = [float]$Size
    $cx = $s / 2.0
    $cy = $s / 2.0
    $w  = $s * 0.30
    $h  = $s * 0.42

    # Dalle arrondie sombre.
    $tile = New-Object System.Drawing.Drawing2D.GraphicsPath
    $rad = $s * 0.22
    $d = $rad * 2.0
    $tile.AddArc(0, 0, $d, $d, 180, 90)
    $tile.AddArc($s - $d, 0, $d, $d, 270, 90)
    $tile.AddArc($s - $d, $s - $d, $d, $d, 0, 90)
    $tile.AddArc(0, $s - $d, $d, $d, 90, 90)
    $tile.CloseFigure()

    $c1 = [System.Drawing.Color]::FromArgb(255, 30, 41, 59)
    $c2 = [System.Drawing.Color]::FromArgb(255, 15, 23, 42)
    $pTop = New-Object System.Drawing.Point -ArgumentList 0, 0
    $pBot = New-Object System.Drawing.Point -ArgumentList 0, $Size
    $tileBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush -ArgumentList $pTop, $pBot, $c1, $c2
    $g.FillPath($tileBrush, $tile)

    # Bouclier vert.
    $pts = [System.Drawing.PointF[]]@(
        (New-Object System.Drawing.PointF ($cx - $w), ($cy - $h * 0.85)),
        (New-Object System.Drawing.PointF ($cx + $w), ($cy - $h * 0.85)),
        (New-Object System.Drawing.PointF ($cx + $w), ($cy - $h * 0.05)),
        (New-Object System.Drawing.PointF ($cx + $w * 0.6), ($cy + $h * 0.25)),
        (New-Object System.Drawing.PointF $cx, ($cy + $h * 0.95)),
        (New-Object System.Drawing.PointF ($cx - $w * 0.6), ($cy + $h * 0.25)),
        (New-Object System.Drawing.PointF ($cx - $w), ($cy - $h * 0.05))
    )
    $shield = New-Object System.Drawing.Drawing2D.GraphicsPath
    $shield.AddPolygon($pts)
    $shield.CloseFigure()

    $yTopI = [int]($cy - $h)
    $yBotI = [int]($cy + $h)
    $g1 = [System.Drawing.Color]::FromArgb(255, 74, 222, 128)
    $g2 = [System.Drawing.Color]::FromArgb(255, 22, 163, 74)
    $pGTop = New-Object System.Drawing.Point -ArgumentList 0, $yTopI
    $pGBot = New-Object System.Drawing.Point -ArgumentList 0, $yBotI
    $gBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush -ArgumentList $pGTop, $pGBot, $g1, $g2
    $g.FillPath($gBrush, $shield)

    $pen = New-Object System.Drawing.Pen -ArgumentList ([System.Drawing.Color]::FromArgb(255, 5, 46, 22)), ([Math]::Max(1.0, $s * 0.02))
    $g.DrawPath($pen, $shield)

    # Coche blanche.
    $pen2 = New-Object System.Drawing.Pen -ArgumentList ([System.Drawing.Color]::White), ([Math]::Max(1.5, $s * 0.07))
    $pen2.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pen2.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $g.DrawLines($pen2, [System.Drawing.PointF[]]@(
        (New-Object System.Drawing.PointF ($cx - $w * 0.42), ($cy - $h * 0.05)),
        (New-Object System.Drawing.PointF ($cx - $w * 0.08), ($cy + $h * 0.22)),
        (New-Object System.Drawing.PointF ($cx + $w * 0.5), ($cy - $h * 0.5))
    ))
    $g.Dispose()
    return $bmp
}

# --- Convertit un bitmap en image DIB 32bpp + masque AND ---------------------
function New-DibImage {
    param([System.Drawing.Bitmap]$Bmp)
    $w = $Bmp.Width; $h = $Bmp.Height
    $ms = New-Object System.IO.MemoryStream
    $bw = New-Object System.IO.BinaryWriter -ArgumentList $ms
    $bw.Write([UInt32]40)
    $bw.Write([Int32]$w)
    $bw.Write([Int32]($h * 2))
    $bw.Write([UInt16]1)
    $bw.Write([UInt16]32)
    $bw.Write([UInt32]0)
    $bw.Write([UInt32]($w * $h * 4))
    $bw.Write([Int32]0); $bw.Write([Int32]0)
    $bw.Write([UInt32]0); $bw.Write([UInt32]0)
    for ($y = $h - 1; $y -ge 0; $y--) {
        for ($x = 0; $x -lt $w; $x++) {
            $px = $Bmp.GetPixel($x, $y)
            $bw.Write([byte]$px.B); $bw.Write([byte]$px.G)
            $bw.Write([byte]$px.R); $bw.Write([byte]$px.A)
        }
    }
    $maskRow = [int][Math]::Ceiling($w / 32.0) * 4
    $bw.Write((New-Object byte[] ($maskRow * $h)))
    $bw.Flush()
    $data = $ms.ToArray()
    $bw.Dispose(); $ms.Dispose()
    return , $data
}

# --- Génération --------------------------------------------------------------
$sizes = @(256, 48, 32, 16)
$images = New-Object System.Collections.ArrayList
foreach ($sz in $sizes) {
    $bmp = New-ShieldBitmap -Size $sz
    $dib = New-DibImage -Bmp $bmp
    Write-Host ("  taille {0} -> DIB {1} octets" -f $sz, $dib.Length)
    [void]$images.Add($dib)
    $bmp.Dispose()
}

$count = $images.Count
$offset = 6 + (16 * $count)
$fs = [System.IO.File]::Create($outFile)
$bw = New-Object System.IO.BinaryWriter -ArgumentList $fs
$bw.Write([UInt16]0)
$bw.Write([UInt16]1)
$bw.Write([UInt16]$count)
for ($i = 0; $i -lt $count; $i++) {
    $sz = $sizes[$i]
    $data = $images[$i]
    $dim = if ($sz -ge 256) { 0 } else { $sz }
    $bw.Write([byte]$dim); $bw.Write([byte]$dim)
    $bw.Write([byte]0); $bw.Write([byte]0)
    $bw.Write([UInt16]1); $bw.Write([UInt16]32)
    $bw.Write([UInt32]$data.Length)
    $bw.Write([UInt32]$offset)
    $offset += $data.Length
}
foreach ($data in $images) { $bw.Write($data) }
$bw.Flush(); $bw.Dispose(); $fs.Dispose()

Write-Host ("Icône générée : {0} ({1} octets)" -f $outFile, (Get-Item $outFile).Length)
