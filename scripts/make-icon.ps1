# Generate LiteTrans app icon source (1024x1024 PNG).
#
# Mark: Latin "A" above, Chinese "文" below, split by a two-headed arrow.
# Alternate scripts side by side is the one visual that says "translation"
# without reading as music, an audio equalizer, or a generic letter tile.
#
# Layout is stacked, not side by side: at 16px (the tray icon) two glyphs
# sharing a row collapse into noise, while stacked glyphs keep a clear
# silhouette. The arrow stays a single horizontal element for the same reason.
#
# Colors come from the app's five-tone palette: shang silver and gong ochre.
# ASCII-only on purpose: PowerShell 5.1 misparses UTF-8 scripts without BOM.
Add-Type -AssemblyName System.Drawing

$size = 1024
$bmp = New-Object System.Drawing.Bitmap($size, $size)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit

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

# Hairline inner edge so the tile holds on both light and dark taskbars
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

$shangColor = [System.Drawing.Color]::FromArgb(255, 200, 209, 222)
$gongColor = [System.Drawing.Color]::FromArgb(255, 215, 169, 88)
$arrowColor = [System.Drawing.Color]::FromArgb(255, 152, 165, 184)

# GenericTypographic measures the glyph's ink, not the padded em box, so the
# two letters can be centered by their actual drawn extent.
$fmtTight = [System.Drawing.StringFormat]::GenericTypographic
$fmtTight.FormatFlags = $fmtTight.FormatFlags -bor [System.Drawing.StringFormatFlags]::MeasureTrailingSpaces
$fmt = New-Object System.Drawing.StringFormat
$fmt.Alignment = [System.Drawing.StringAlignment]::Center
$fmt.LineAlignment = [System.Drawing.StringAlignment]::Center

$fontSize = 380
$fontA = New-Object System.Drawing.Font('Segoe UI', $fontSize, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$fontWen = New-Object System.Drawing.Font('Microsoft YaHei UI', $fontSize, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)

$szA = $g.MeasureString('A', $fontA, 2000, $fmtTight)
$szWen = $g.MeasureString([string][char]0x6587, $fontWen, 2000, $fmtTight)

# Stack: A over arrow over 文, each centered on the plate's vertical axis.
# Optical weight: the arrow sits closer to the lower glyph so the block reads
# as "A becomes 文" rather than as two unrelated marks.
$cx = $size / 2
$cyA = 322
$arrowY = 534
$cyWen = 762

$boxA = New-Object System.Drawing.RectangleF(($cx - $szA.Width / 2), ($cyA - $szA.Height / 2), $szA.Width, $szA.Height)
$boxWen = New-Object System.Drawing.RectangleF(($cx - $szWen.Width / 2), ($cyWen - $szWen.Height / 2), $szWen.Width, $szWen.Height)

$brushA = New-Object System.Drawing.SolidBrush($shangColor)
$brushWen = New-Object System.Drawing.SolidBrush($gongColor)
$g.DrawString('A', $fontA, $brushA, $boxA, $fmt)
$g.DrawString([string][char]0x6587, $fontWen, $brushWen, $boxWen, $fmt)
$brushA.Dispose(); $brushWen.Dispose()

# Two-headed arrow: bidirectional translation, not one-way.
# Width is clamped to the wider glyph so the block keeps one silhouette.
$blockW = [Math]::Max($szA.Width, $szWen.Width)
$ax1 = $cx - $blockW / 2 - 18
$ax2 = $cx + $blockW / 2 + 18
$shaft = 22
$headLen = 52
$headHalf = 38

$arrowBrush = New-Object System.Drawing.SolidBrush($arrowColor)
$shaftRect = New-Object System.Drawing.RectangleF(($ax1 + $headLen), ($arrowY - $shaft / 2), (($ax2 - $ax1) - 2 * $headLen), $shaft)
$g.FillRectangle($arrowBrush, $shaftRect)

$headL = New-Object System.Drawing.Drawing2D.GraphicsPath
$headL.AddPolygon(@(
    (New-Object System.Drawing.PointF($ax1, $arrowY)),
    (New-Object System.Drawing.PointF(($ax1 + $headLen), ($arrowY - $headHalf))),
    (New-Object System.Drawing.PointF(($ax1 + $headLen), ($arrowY + $headHalf)))
))
$g.FillPath($arrowBrush, $headL)
$headL.Dispose()

$headR = New-Object System.Drawing.Drawing2D.GraphicsPath
$headR.AddPolygon(@(
    (New-Object System.Drawing.PointF($ax2, $arrowY)),
    (New-Object System.Drawing.PointF(($ax2 - $headLen), ($arrowY - $headHalf))),
    (New-Object System.Drawing.PointF(($ax2 - $headLen), ($arrowY + $headHalf)))
))
$g.FillPath($arrowBrush, $headR)
$headR.Dispose()
$arrowBrush.Dispose()

$bmp.Save('D:\LiteTrans\app-icon.png', [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bg.Dispose(); $plate.Dispose(); $bmp.Dispose()
$fontA.Dispose(); $fontWen.Dispose(); $fmt.Dispose(); $fmtTight.Dispose()
Write-Output "icon written: D:\LiteTrans\app-icon.png"
