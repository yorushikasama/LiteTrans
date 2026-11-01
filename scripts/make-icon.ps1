# Generate LiteTrans app icon source (1024x1024 PNG).
#
# Mark: a large ink "文" on a gong-ochre plate, with a small Latin "A" tucked
# into the lower-left corner. Primary + secondary reads as "this turns into
# that" in one glance; two equally weighted glyphs (the previous version)
# read as two unrelated symbols sharing a tile.
#
# Color: the plate IS the app's primary color (gong ochre, --gong in the theme)
# instead of the dark plate every version before this used. A warm solid fill
# is what makes an icon pop on both light and dark taskbars, and it matches
# the accent the app already uses for primary actions.
#
# ASCII-only on purpose: PowerShell 5.1 misparses UTF-8 scripts without BOM.
Add-Type -AssemblyName System.Drawing

$size = 1024
$bmp = New-Object System.Drawing.Bitmap($size, $size)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit

# Rounded-square plate, Windows tile proportions
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

# Gong ochre, with a slight vertical falloff so the tile has body without
# turning into a glossy button
$bg = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    $rect,
    [System.Drawing.Color]::FromArgb(255, 231, 181, 81),
    [System.Drawing.Color]::FromArgb(255, 205, 152, 55),
    90.0)
$g.FillPath($bg, $plate)

$inkColor = [System.Drawing.Color]::FromArgb(255, 26, 33, 45)
$inkSoft = [System.Drawing.Color]::FromArgb(255, 26, 33, 45)

$fmtTight = [System.Drawing.StringFormat]::GenericTypographic
$fmtTight.FormatFlags = $fmtTight.FormatFlags -bor [System.Drawing.StringFormatFlags]::MeasureTrailingSpaces
$fmt = New-Object System.Drawing.StringFormat
$fmt.Alignment = [System.Drawing.StringAlignment]::Center
$fmt.LineAlignment = [System.Drawing.StringAlignment]::Center

# Primary glyph: 文, sized to dominate. Inset from the plate edge so the
# silhouette is a rounded square of color, not a square jammed with ink.
$bigSize = 452
$fontWen = New-Object System.Drawing.Font('Microsoft YaHei UI', $bigSize, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$szWen = $g.MeasureString([string][char]0x6587, $fontWen, 2000, $fmtTight)

# 文 sits right-of-center and slightly high: the small A takes the lower-left
$wenCx = 588
$wenCy = 408
$boxWen = New-Object System.Drawing.RectangleF(($wenCx - $szWen.Width / 2), ($wenCy - $szWen.Height / 2), $szWen.Width, $szWen.Height)
$inkBrush = New-Object System.Drawing.SolidBrush($inkColor)
$g.DrawString([string][char]0x6587, $fontWen, $inkBrush, $boxWen, $fmt)
$inkBrush.Dispose()

# Secondary glyph: A in a small ink chip, bottom-left. The chip is a solid
# shape rather than bare letterform so it survives downscaling and reads as
# a deliberate counterweight to the big 文.
$chipSize = 232
$chipX = 196
$chipY = 668
$chipR = 58
$chip = New-Object System.Drawing.Drawing2D.GraphicsPath
$chip.AddArc($chipX, $chipY, ($chipR * 2), ($chipR * 2), 180, 90)
$chip.AddArc(($chipX + $chipSize - $chipR * 2), $chipY, ($chipR * 2), ($chipR * 2), 270, 90)
$chip.AddArc(($chipX + $chipSize - $chipR * 2), ($chipY + $chipSize - $chipR * 2), ($chipR * 2), ($chipR * 2), 0, 90)
$chip.AddArc($chipX, ($chipY + $chipSize - $chipR * 2), ($chipR * 2), ($chipR * 2), 90, 90)
$chip.CloseFigure()
$chipBrush = New-Object System.Drawing.SolidBrush($inkSoft)
$g.FillPath($chipBrush, $chip)
$chipBrush.Dispose()

$smallSize = 168
$fontA = New-Object System.Drawing.Font('Segoe UI', $smallSize, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$szA = $g.MeasureString('A', $fontA, 2000, $fmtTight)
$boxA = New-Object System.Drawing.RectangleF(
    ($chipX + $chipSize / 2 - $szA.Width / 2),
    ($chipY + $chipSize / 2 - $szA.Height / 2),
    $szA.Width, $szA.Height)
$lightBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 240, 205, 138))
$g.DrawString('A', $fontA, $lightBrush, $boxA, $fmt)
$lightBrush.Dispose()

# Two short swap arrows in the gap to the right of the A chip, stacked:
# upper points right (toward 文), lower points left (back toward A).
# Kept out of the 文 捺 stroke so nothing overlaps.
$arrowBrush = New-Object System.Drawing.SolidBrush($inkSoft)
$aw = 132
$ah = 16
$ax1 = 496
$ax2 = 676
$ay1 = 700
$ay2 = 762
$hLen = 30
$hHalf = 22

# upper arrow: shaft + right head
$g.FillRectangle($arrowBrush, (New-Object System.Drawing.RectangleF($ax1, ($ay1 - $ah / 2), ($ax2 - $ax1 - $hLen), $ah)))
$p1 = New-Object System.Drawing.Drawing2D.GraphicsPath
$p1.AddPolygon(@(
    (New-Object System.Drawing.PointF($ax2, $ay1)),
    (New-Object System.Drawing.PointF(($ax2 - $hLen), ($ay1 - $hHalf))),
    (New-Object System.Drawing.PointF(($ax2 - $hLen), ($ay1 + $hHalf)))
))
$g.FillPath($arrowBrush, $p1); $p1.Dispose()

# lower arrow: shaft + left head
$g.FillRectangle($arrowBrush, (New-Object System.Drawing.RectangleF(($ax1 + $hLen), ($ay2 - $ah / 2), ($ax2 - $ax1 - $hLen), $ah)))
$p2 = New-Object System.Drawing.Drawing2D.GraphicsPath
$p2.AddPolygon(@(
    (New-Object System.Drawing.PointF($ax1, $ay2)),
    (New-Object System.Drawing.PointF(($ax1 + $hLen), ($ay2 - $hHalf))),
    (New-Object System.Drawing.PointF(($ax1 + $hLen), ($ay2 + $hHalf)))
))
$g.FillPath($arrowBrush, $p2); $p2.Dispose()
$arrowBrush.Dispose()

$bmp.Save('D:\LiteTrans\app-icon.png', [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bg.Dispose(); $plate.Dispose(); $bmp.Dispose()
$fontWen.Dispose(); $fontA.Dispose(); $fmt.Dispose(); $fmtTight.Dispose(); $chip.Dispose()
Write-Output "icon written: D:\LiteTrans\app-icon.png"
