# Installe llama.cpp b8184 CPU pour Blin / Loggy.
param(
    [string]$InstallDir = ""
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
if ($InstallDir) {
    $destRoot = $InstallDir
    if (-not (Test-Path $destRoot)) { New-Item -ItemType Directory -Path $destRoot -Force | Out-Null }
    $dest = Join-Path $destRoot "llama-b8184-bin-win-cpu-x64"
} else {
    $dest = Join-Path $root "llama-b8184-bin-win-cpu-x64"
}
$zipLlama = Join-Path $env:TEMP "llama-b8184-bin-win-cpu-x64.zip"
$base = "https://github.com/ggml-org/llama.cpp/releases/download/b8184"

Write-Host "Blin - installation llama-server CPU b8184"
if (-not (Test-Path $dest)) { New-Item -ItemType Directory -Path $dest | Out-Null }

if (Test-Path (Join-Path $dest "llama-server.exe")) {
    Write-Host "llama-server deja present : $dest"
    exit 0
}

Write-Host "Telechargement llama CPU, environ 80 Mo..."
Invoke-WebRequest -Uri "$base/llama-b8184-bin-win-cpu-x64.zip" -OutFile $zipLlama -UseBasicParsing
Expand-Archive -Path $zipLlama -DestinationPath $dest -Force

# Certains zips deploient dans un sous-dossier du meme nom.
$nested = Join-Path $dest "llama-b8184-bin-win-cpu-x64"
if ((Test-Path $nested) -and (Test-Path (Join-Path $nested "llama-server.exe"))) {
    Get-ChildItem -Path $nested -Force | Move-Item -Destination $dest -Force
    Remove-Item -Path $nested -Recurse -Force -ErrorAction SilentlyContinue
}

if (-not (Test-Path (Join-Path $dest "llama-server.exe"))) {
    Write-Error "llama-server.exe introuvable apres extraction dans $dest"
}

Write-Host "OK - CPU pret : $dest"
