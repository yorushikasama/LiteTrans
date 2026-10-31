# Generate LiteTrans app icon source (1024x1024 PNG): five-tone mark.
# Five vertical bars (gong shang jue zhi yu), the active tone in gong-ochre.
# ASCII-only on purpose: PowerShell 5.1 misparses UTF-8 scripts without BOM.
Add-Type -AssemblyName System.Drawing

$size = 1024
$bmp = New-Object System.Drawing.Bitmap($size, $size)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias

# Rounded-square plate (ink-cool near black), Windows tile proportions
$pad = 24
$rect = New-Object System.Drawing.Rectangle($pad, $pad, ($size - 2 * $pad), ($size - 2 * $pad))
$radius = 226
$d = $radius * 2
$path = New-Object System.Drawing.Drawing2D.GraphicsPath
$path.AddArc($rect.X, $rect.Y, $d, $d, 180, 90)
$path.AddArc(($rect.Right - $d), $rect.Y, $d, $d, 270, 90)
$path.AddArc(($rect.Right - $d), ($rect.Bottom - $d), $d, $d, 0, 90)
$path.AddArc($rect.X, ($rect.Bottom - $d), $d, $d, 90, 90)
$path.CloseFigure()

# Solid ink plate with a faint vertical falloff (no decorative rainbow)
$plate = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    $rect,
    [System.Drawing.Color]::FromArgb(255, 33, 41, 55),
    [System.Drawing.Color]::FromArgb(255, 19, 25, 35),
    90.0)
$g.FillPath($plate, $path)

# Five-line staff: five lines ARE the five tones. One gong-colored note head
# sits on the staff with a stem. A staff cannot be misread as a bar chart, and
# "five lines + one struck note" says pentatonic without any ornament.
$lineX = 176
$lineW = $size - 2 * $lineX
$lineH = 15
$gapY = 62
$staffH = 4 * $gapY + $lineH
# Optical center: stem rises 174px above the staff, staff extends 263px down,
# so the composition center sits 44px below the staff top.
$y0 = 468
$gongColor = [System.Drawing.Color]::FromArgb(255, 216, 170, 90)
$mutedColor = [System.Drawing.Color]::FromArgb(255, 147, 160, 179)

for ($i = 0; $i -lt 5; $i++) {
    $y = $y0 + $i * $gapY
    $r = $lineH / 2
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath
    $p.AddArc($lineX, $y, $lineH, $lineH, 90, 180)
    $p.AddArc(($lineX + $lineW - $lineH), $y, $lineH, $lineH, 270, 180)
    $p.CloseFigure()
    $b = New-Object System.Drawing.SolidBrush($mutedColor)
    $g.FillPath($b, $p)
    $b.Dispose(); $p.Dispose()
}

# Note head on the second line from the top (the "shang" degree), stem up
$headR = 52
$cx = $lineX + $lineW * 0.42
$cy = $y0 + $gapY
$gb = New-Object System.Drawing.SolidBrush($gongColor)
$g.FillEllipse($gb, ($cx - $headR), ($cy - $headR + 12), ($headR * 2 + 14), ($headR * 2 - 24))

# Stem
$stemW = 17
$stemTop = $cy - 236
$stem = New-Object System.Drawing.Rectangle([int]($cx + $headR - 4), [int]$stemTop, $stemW, [int]($cy - $stemTop + 8))
$g.FillRectangle($gb, $stem)
$gb.Dispose()

$bmp.Save('D:\LiteTrans\app-icon.png', [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $plate.Dispose(); $bmp.Dispose()
Write-Output "icon written: D:\LiteTrans\app-icon.png"
