# Genere les icones Tauri (barre des taches, installeur).
# Priorite : 1er argument (ex. ecosystem-icon.png du registre), sinon public/logo.png
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$srcLogo = if ($args.Count -gt 0 -and (Test-Path $args[0])) { $args[0] } else { Join-Path $root "public\logo.png" }
$appIcon = Join-Path $root "src-tauri\app-icon.png"

if (-not (Test-Path $srcLogo)) {
    Write-Error "Fichier introuvable : $srcLogo"
}

Add-Type -AssemblyName System.Drawing
$src = [System.Drawing.Image]::FromFile($srcLogo)
$targetSize = 1024
$bmp = New-Object System.Drawing.Bitmap $targetSize, $targetSize
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.Clear([System.Drawing.Color]::Black)
$scale = [Math]::Min($targetSize / $src.Width, $targetSize / $src.Height)
$w = [int]($src.Width * $scale)
$h = [int]($src.Height * $scale)
$x = [int](($targetSize - $w) / 2)
$y = [int](($targetSize - $h) / 2)
$g.DrawImage($src, $x, $y, $w, $h)
$bmp.Save($appIcon, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()
$src.Dispose()

Write-Host "Icone source : $appIcon (1024x1024)"
Push-Location $root
try {
    npx tauri icon $appIcon
    Write-Host "Icones Tauri regenerees dans src-tauri/icons/"
} finally {
    Pop-Location
}
