# Download Ministral 8B GGUF (strong French) for Blin / Loggy.
param(
    [string]$InstallDir = ""
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
if ($InstallDir) {
    $bundleDir = Join-Path $InstallDir "llama-b8184-bin-win-cpu-x64\Ministral-8B-Instruct-2410-GGUF"
} else {
    $bundleDir = Join-Path $root "llama-b8184-bin-win-cpu-x64\Ministral-8B-Instruct-2410-GGUF"
}
$destFile = Join-Path $bundleDir "Ministral-8B-Instruct-2410.Q5_K_S.gguf"
$url = "https://huggingface.co/mradermacher/Ministral-8B-Instruct-2410-GGUF/resolve/main/Ministral-8B-Instruct-2410.Q5_K_S.gguf"

if (-not (Test-Path $bundleDir)) {
    New-Item -ItemType Directory -Path $bundleDir -Force | Out-Null
}

if (Test-Path $destFile) {
    $size = (Get-Item $destFile).Length
    if ($size -gt 5GB) {
        Write-Host "Model already present: $destFile"
        exit 0
    }
    Write-Host "Incomplete file, re-downloading..."
    Remove-Item $destFile -Force
}

Write-Host "Blin - downloading Ministral 8B Instruct Q5_K_S (~5.2 GB)"
Write-Host "Source: mradermacher/Ministral-8B-Instruct-2410-GGUF"
Write-Host "Destination: $destFile"

$tmp = "$destFile.part"
if (Test-Path $tmp) {
    $partSize = (Get-Item $tmp).Length
    Write-Host "Reprise du telechargement ($([math]::Round($partSize/1GB, 2)) Go deja presents)..."
}

if (Get-Command curl.exe -ErrorAction SilentlyContinue) {
    & curl.exe -L --retry 5 --retry-delay 3 -C - -o $tmp $url
    Move-Item -Path $tmp -Destination $destFile -Force
} elseif (Get-Command Start-BitsTransfer -ErrorAction SilentlyContinue) {
    Start-BitsTransfer -Source $url -Destination $tmp -DisplayName "Ministral-8B-GGUF"
    Move-Item -Path $tmp -Destination $destFile -Force
} else {
    Invoke-WebRequest -Uri $url -OutFile $tmp -UseBasicParsing
    Move-Item -Path $tmp -Destination $destFile -Force
}

Write-Host "OK - model installed. Restart Blin (npm run tauri dev)."
