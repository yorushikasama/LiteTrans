# Generate LiteTrans app icon source (1024x1024 PNG).
#
# Mark: the tile is split along the anti-diagonal into an ink half and a gong
# ochre half. 文 sits in the ink half knocked out in ochre; A sits in the
# ochre half knocked out in ink. Each script lives inside the other's color,
# which is what translation IS, so no arrow is needed to say "convert".
# This diagonal two-glyph lockup is the composition Google Translate made the
# canonical translation mark; the palette here is the app's own (gong ochre +
# ink), which keeps it from being a GT clone.
#
# Earlier attempts failed for concrete reasons: a staff read as music, five
# strings as an equalizer, stacked A/arrow/文 as three disconnected bands, a
# corner A chip as a notification badge, and a diagonal arrow collided with
# 文's strokes. Two glyphs, two colors, zero third elements is the end state.
#
# ASCII-only on purpose: PowerShell 5.1 misparses UTF-8 scripts without BOM.
Add-Type -AssemblyName System.Drawing

$size = 1024
$bmp = New-Object System.Drawing.Bitmap($size, $size)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit

$ochre = [System.Drawing.Color]::FromArgb(255, 231, 181, 81)
$ink = [System.Drawing.Color]::FromArgb(255, 26, 33, 45)

# Rounded-square plate, Windows tile proportions, filled ochre
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
$plateBrush = New-Object System.Drawing.SolidBrush($ochre)
$g.FillPath($plateBrush, $plate)

# Ink half: everything above/left of the anti-diagonal (top-right corner to
# bottom-left corner), clipped to the plate so the rounded corners survive.
$g.SetClip($plate)
$inkBrush = New-Object System.Drawing.SolidBrush($ink)
$half = New-Object System.Drawing.Drawing2D.GraphicsPath
$half.AddPolygon(@(
    (New-Object System.Drawing.PointF(($rect.Right + 4), ($rect.Top - 4))),
    (New-Object System.Drawing.PointF(($rect.Left - 4), ($rect.Top - 4))),
    (New-Object System.Drawing.PointF(($rect.Left - 4), ($rect.Bottom + 4)))
))
$g.FillPath($inkBrush, $half)
$half.Dispose()

$fmtTight = [System.Drawing.StringFormat]::GenericTypographic
$fmtTight.FormatFlags = $fmtTight.FormatFlags -bor [System.Drawing.StringFormatFlags]::MeasureTrailingSpaces
$fmt = New-Object System.Drawing.StringFormat
$fmt.Alignment = [System.Drawing.StringAlignment]::Center
$fmt.LineAlignment = [System.Drawing.StringAlignment]::Center

# 文 in the ink half, knocked out in ochre. Kept clear of the diagonal
# (points with x + y near 1024) so no stroke tip bleeds into the other half.
$fontWen = New-Object System.Drawing.Font('Microsoft YaHei UI', 330, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$szWen = $g.MeasureString([string][char]0x6587, $fontWen, 2000, $fmtTight)
$wenCx = 306
$wenCy = 306
$boxWen = New-Object System.Drawing.RectangleF(($wenCx - $szWen.Width / 2), ($wenCy - $szWen.Height / 2), $szWen.Width, $szWen.Height)
$g.DrawString([string][char]0x6587, $fontWen, $plateBrush, $boxWen, $fmt)

# A in the ochre half, knocked out in ink
$fontA = New-Object System.Drawing.Font('Segoe UI', 330, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$szA = $g.MeasureString('A', $fontA, 2000, $fmtTight)
$aCx = 712
$aCy = 712
$boxA = New-Object System.Drawing.RectangleF(($aCx - $szA.Width / 2), ($aCy - $szA.Height / 2), $szA.Width, $szA.Height)
$g.DrawString('A', $fontA, $inkBrush, $boxA, $fmt)

$g.ResetClip()
$bmp.Save('D:\LiteTrans\app-icon.png', [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $plateBrush.Dispose(); $inkBrush.Dispose(); $plate.Dispose(); $bmp.Dispose()
$fontWen.Dispose(); $fontA.Dispose(); $fmt.Dispose(); $fmtTight.Dispose()
Write-Output "icon written: D:\LiteTrans\app-icon.png"
