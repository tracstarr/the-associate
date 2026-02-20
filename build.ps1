#!/usr/bin/env pwsh
# Build assoc.exe via Docker and copy it to target/x86_64-pc-windows-gnu/release/.
# Usage: ./build.ps1

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$outDir = Join-Path $scriptDir "target\x86_64-pc-windows-gnu\release"

if (-not (Test-Path $outDir)) {
    New-Item -ItemType Directory -Path $outDir -Force | Out-Null
}

Write-Host "==> Building assoc-build image (builder stage)..." -ForegroundColor Cyan
docker build -t assoc-build --target builder $scriptDir

Write-Host "==> Exporting assoc.exe to $outDir ..." -ForegroundColor Cyan
$env:DOCKER_BUILDKIT = 1
docker build --target export --output "type=local,dest=$outDir" $scriptDir

Write-Host "==> Done: $outDir\assoc.exe" -ForegroundColor Green
