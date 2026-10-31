# Generate LiteTrans app icon source (1024x1024 PNG, gradient rounded rect + CJK glyph U+8BD1)
# ASCII-only on purpose: PowerShell 5.1 misparses UTF-8 scripts without BOM.
Add-Type -AssemblyName System.Drawing

$glyph = [char]0x8BD1  # "yi" (translate)

$size = 1024
$bmp = New-Object System.Drawing.Bitmap($size, $size)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit

$rect = New-Object System.Drawing.Rectangle(24, 24, 976, 976)
$radius = 224
$d = $radius * 2
$path = New-Object System.Drawing.Drawing2D.GraphicsPath
$path.AddArc($rect.X, $rect.Y, $d, $d, 180, 90)
$path.AddArc(($rect.Right - $d), $rect.Y, $d, $d, 270, 90)
$path.AddArc(($rect.Right - $d), ($rect.Bottom - $d), $d, $d, 0, 90)
$path.AddArc($rect.X, ($rect.Bottom - $d), $d, $d, 90, 90)
$path.CloseFigure()

$brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    $rect,
    [System.Drawing.Color]::FromArgb(255, 99, 102, 241),
    [System.Drawing.Color]::FromArgb(255, 14, 116, 235),
    55.0)
$g.FillPath($brush, $path)

$font = New-Object System.Drawing.Font('Microsoft YaHei UI', 560, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$fmt = New-Object System.Drawing.StringFormat
$fmt.Alignment = [System.Drawing.StringAlignment]::Center
$fmt.LineAlignment = [System.Drawing.StringAlignment]::Center
$box = New-Object System.Drawing.RectangleF(24, 10, 976, 976)
$g.DrawString($glyph, $font, [System.Drawing.Brushes]::White, $box, $fmt)

$bmp.Save('D:\LiteTrans\app-icon.png', [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $brush.Dispose(); $bmp.Dispose()
Write-Output "icon written: D:\LiteTrans\app-icon.png"
