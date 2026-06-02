# Installe llama.cpp b8184 CUDA 13.1 pour GPU NVIDIA (RTX, etc.)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$dest = Join-Path $root "llama-b8184-bin-win-cuda-13.1-x64"
$zipLlama = Join-Path $env:TEMP "llama-b8184-bin-win-cuda-13.1-x64.zip"
$zipCuda = Join-Path $env:TEMP "cudart-llama-bin-win-cuda-13.1-x64.zip"
$base = "https://github.com/ggml-org/llama.cpp/releases/download/b8184"

Write-Host "Blin — installation llama-server CUDA 13.1"
if (-not (Test-Path $dest)) { New-Item -ItemType Directory -Path $dest | Out-Null }

if (-not (Test-Path (Join-Path $dest "llama-server.exe"))) {
    Write-Host 'Telechargement llama CUDA, environ 145 Mo...'
    Invoke-WebRequest -Uri "$base/llama-b8184-bin-win-cuda-13.1-x64.zip" -OutFile $zipLlama -UseBasicParsing
    Expand-Archive -Path $zipLlama -DestinationPath $dest -Force
}

if (-not (Test-Path (Join-Path $dest "ggml-cuda.dll"))) {
    Write-Host 'Telechargement runtime CUDA, environ 393 Mo...'
    Invoke-WebRequest -Uri "$base/cudart-llama-bin-win-cuda-13.1-x64.zip" -OutFile $zipCuda -UseBasicParsing
    Expand-Archive -Path $zipCuda -DestinationPath $dest -Force
}

Write-Host "OK — GPU pret : $dest"
Write-Host "Relancez Blin (npm run tauri dev)."
