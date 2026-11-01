# Generate LiteTrans app icon source (1024x1024 PNG): five colored strings.
# Five horizontal strings, graduated thickness, each in its own tone color:
# gong (earth ochre), shang (gold white), jue (wood green), zhi (fire red), yu (ink).
# The five colors ARE the theme, so this survives downscaling to a 16px tray icon.
# ASCII-only on purpose: PowerShell 5.1 misparses UTF-8 scripts without BOM.
Add-Type -AssemblyName System.Drawing

$size = 1024
$bmp = New-Object System.Drawing.Bitmap($size, $size)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias

# Ink plate: rounded square, Windows tile proportions
$pad = 24
$rect = New-Object System.Drawing.Rectangle($pad, $pad, ($size - 2 * $pad), ($size - 2 * $pad))
$radius = 226
$d = $radius * 2
$plate = New-Object System.Drawing.Drawing2D.GraphicsPath
$plate.AddArc($rect.X, $rect.Y, $d, $d, 180, 90)
$plate.AddArc(($rect.Right - $d), $rect.Y, $d, $d, 270, 90)
$plate.AddArc(($rect.Right - $d), ($rect.Bottom - $d), $d, $d, 0, 90)
$plate.AddArc($rect.X, ($rect.Bottom - $d), $d, $d, 90, 90)
$plate.CloseFigure()

$bg = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    $rect,
    [System.Drawing.Color]::FromArgb(255, 31, 39, 52),
    [System.Drawing.Color]::FromArgb(255, 18, 24, 34),
    90.0)
$g.FillPath($bg, $plate)

# Hairline inner edge so the plate reads as a solid tile on both light and dark
# taskbars instead of dissolving into the background
$edgePen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(46, 255, 255, 255), 3)
$eInset = $pad + 6
$eR = $radius - 6
$ePath = New-Object System.Drawing.Drawing2D.GraphicsPath
$ePath.AddArc($eInset, $eInset, ($eR * 2), ($eR * 2), 180, 90)
$ePath.AddArc(($size - $eInset - $eR * 2), $eInset, ($eR * 2), ($eR * 2), 270, 90)
$ePath.AddArc(($size - $eInset - $eR * 2), ($size - $eInset - $eR * 2), ($eR * 2), ($eR * 2), 0, 90)
$ePath.AddArc($eInset, ($size - $eInset - $eR * 2), ($eR * 2), ($eR * 2), 90, 90)
$ePath.CloseFigure()
$g.DrawPath($edgePen, $ePath)
$edgePen.Dispose(); $ePath.Dispose()

# Five vertical strings in the five tone colors, hung from a bridge bar.
# Vertical + bridge reads as a strung instrument rather than a color swatch stack,
# and five distinct hues survive downscaling to a 16px tray icon where a single
# glyph or staff would turn to mud.
$colors = @(
    @(215, 169, 88),   # gong  earth ochre
    @(200, 209, 222),  # shang gold white
    @(110, 181, 160),  # jue   wood green
    @(201, 93, 80),    # zhi   fire red
    @(126, 144, 180)   # yu    ink blue
)
$widths = @(112, 94, 78, 62, 48)
$gap = 22
$totalW = 0
foreach ($w in $widths) { $totalW += $w }
$totalW += 4 * $gap
$x = ($size - $totalW) / 2
$stringTop = 210
$stringBottom = 878

# Bridge bar across the top of the strings (yueshan)
$bridgeH = 34
$bridgePad = 26
$bx = $x - $bridgePad
$bw = $totalW + 2 * $bridgePad
$bY = $stringTop - $bridgeH + 6
$bp = New-Object System.Drawing.Drawing2D.GraphicsPath
$bp.AddArc($bx, $bY, $bridgeH, $bridgeH, 90, 180)
$bp.AddArc(($bx + $bw - $bridgeH), $bY, $bridgeH, $bridgeH, 270, 180)
$bp.CloseFigure()
$bb = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 168, 180, 196))
$g.FillPath($bb, $bp)
$bb.Dispose(); $bp.Dispose()

for ($i = 0; $i -lt 5; $i++) {
    $w = $widths[$i]
    $c = $colors[$i]
    $h = $stringBottom - $stringTop
    $r = [Math]::Min(20, $w / 2)
    $dd = $r * 2
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath
    $p.AddArc($x, $stringTop, $dd, $dd, 180, 90)
    $p.AddArc(($x + $w - $dd), $stringTop, $dd, $dd, 270, 90)
    $p.AddArc(($x + $w - $dd), ($stringTop + $h - $dd), $dd, $dd, 0, 90)
    $p.AddArc($x, ($stringTop + $h - $dd), $dd, $dd, 90, 90)
    $p.CloseFigure()
    $brush = New-Object System.Drawing.SolidBrush(
        [System.Drawing.Color]::FromArgb(255, $c[0], $c[1], $c[2]))
    $g.FillPath($brush, $p)
    $brush.Dispose(); $p.Dispose()
    $x += $w + $gap
}

$bmp.Save('D:\LiteTrans\app-icon.png', [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bg.Dispose(); $plate.Dispose(); $bmp.Dispose()
Write-Output "icon written: D:\LiteTrans\app-icon.png"
