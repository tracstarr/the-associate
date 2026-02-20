#!/usr/bin/env pwsh
# The Associate installer (Windows)
# Usage: irm https://raw.githubusercontent.com/tracstarr/the-associate/main/install.ps1 | iex

$ErrorActionPreference = "Stop"
$repo = "tracstarr/the-associate"

Write-Host "Installing The Associate..." -ForegroundColor Cyan

# Get latest release from GitHub API
$releaseUrl = "https://api.github.com/repos/$repo/releases/latest"
try {
    $release = Invoke-RestMethod -Uri $releaseUrl -Headers @{ Accept = "application/vnd.github+json" }
} catch {
    Write-Error "Failed to fetch latest release: $_"
    return
}

$asset = $release.assets | Where-Object { $_.name -eq "assoc.exe" }
if (-not $asset) {
    Write-Error "Asset 'assoc.exe' not found in release $($release.tag_name)."
    return
}

$downloadUrl = $asset.browser_download_url
$version = $release.tag_name

Write-Host "Downloading assoc.exe ($version)..." -ForegroundColor Cyan

# Install to %LOCALAPPDATA%\Programs\assoc
$installDir = Join-Path $env:LOCALAPPDATA "Programs\assoc"
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

$binaryPath = Join-Path $installDir "assoc.exe"
Invoke-WebRequest -Uri $downloadUrl -OutFile $binaryPath -UseBasicParsing

Write-Host "Installed to $binaryPath" -ForegroundColor Green

# Add to user PATH if not already there
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
    $env:Path = "$env:Path;$installDir"
    Write-Host "Added $installDir to user PATH." -ForegroundColor Green
    Write-Host "Restart your terminal for PATH changes to take effect." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Done! Run 'assoc' to start The Associate." -ForegroundColor Green
