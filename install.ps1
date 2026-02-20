#!/usr/bin/env pwsh
# The Associate installer (Windows)
#
# Install or update:
#   irm https://raw.githubusercontent.com/tracstarr/the-associate/main/install.ps1 | iex
#
# Uninstall:
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/tracstarr/the-associate/main/install.ps1))) -Uninstall

param(
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"
$repo = "tracstarr/the-associate"
$installDir = Join-Path $env:LOCALAPPDATA "bin"
$binaryPath = Join-Path $installDir "assoc.exe"

# Legacy install location (< v0.5)
$legacyDir = Join-Path $env:LOCALAPPDATA "Programs\assoc"
$legacyBinary = Join-Path $legacyDir "assoc.exe"

function Remove-FromPath {
    param([string]$Dir)
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $entries = $userPath -split ';' | Where-Object { $_.TrimEnd('\') -ne $Dir.TrimEnd('\') -and $_ -ne '' }
    $newPath = $entries -join ';'
    if ($newPath -ne $userPath) {
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        return $true
    }
    return $false
}

function Add-ToPath {
    param([string]$Dir)
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $entries = $userPath -split ';' | ForEach-Object { $_.TrimEnd('\') }
    if ($entries -notcontains $Dir.TrimEnd('\')) {
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$Dir", "User")
        $env:Path = "$env:Path;$Dir"
        Write-Host "Added $Dir to user PATH." -ForegroundColor Green
        Write-Host "Restart your terminal for PATH changes to take effect." -ForegroundColor Yellow
    }
}

function Remove-LegacyInstall {
    if (Test-Path $legacyBinary) {
        Write-Host "Removing legacy install from $legacyDir..." -ForegroundColor Yellow
        Remove-Item $legacyBinary -Force
        # Remove the directory if empty
        if ((Get-ChildItem $legacyDir -Force | Measure-Object).Count -eq 0) {
            Remove-Item $legacyDir -Force
        }
        Remove-FromPath $legacyDir | Out-Null
    }
}

function Get-LatestRelease {
    $releaseUrl = "https://api.github.com/repos/$repo/releases/latest"
    try {
        $release = Invoke-RestMethod -Uri $releaseUrl -Headers @{ Accept = "application/vnd.github+json" }
    } catch {
        Write-Error "Failed to fetch latest release: $_"
        return $null
    }

    $asset = $release.assets | Where-Object { $_.name -eq "assoc.exe" }
    if (-not $asset) {
        Write-Error "Asset 'assoc.exe' not found in release $($release.tag_name)."
        return $null
    }

    return @{
        Version     = $release.tag_name
        DownloadUrl = $asset.browser_download_url
    }
}

# === UNINSTALL ===
if ($Uninstall) {
    Write-Host "Uninstalling The Associate..." -ForegroundColor Cyan

    $found = $false

    if (Test-Path $binaryPath) {
        Remove-Item $binaryPath -Force
        Write-Host "Removed $binaryPath" -ForegroundColor Green
        $found = $true
    }

    if (Test-Path $legacyBinary) {
        Remove-Item $legacyBinary -Force
        if ((Get-ChildItem $legacyDir -Force | Measure-Object).Count -eq 0) {
            Remove-Item $legacyDir -Force
        }
        Write-Host "Removed legacy install from $legacyDir" -ForegroundColor Green
        $found = $true
    }

    # Clean PATH entries
    $removedPath = (Remove-FromPath $installDir) -or (Remove-FromPath $legacyDir)
    if ($removedPath) {
        Write-Host "Cleaned PATH entries. Restart your terminal for changes to take effect." -ForegroundColor Yellow
    }

    if (-not $found) {
        Write-Host "The Associate is not installed." -ForegroundColor Yellow
    } else {
        Write-Host ""
        Write-Host "The Associate has been uninstalled." -ForegroundColor Green
    }
    return
}

# === INSTALL / UPDATE ===
$isUpdate = Test-Path $binaryPath

if ($isUpdate) {
    Write-Host "Updating The Associate..." -ForegroundColor Cyan
} else {
    Write-Host "Installing The Associate..." -ForegroundColor Cyan
}

$release = Get-LatestRelease
if (-not $release) { return }

Write-Host "Downloading assoc.exe ($($release.Version))..." -ForegroundColor Cyan

# Create install directory
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

Invoke-WebRequest -Uri $release.DownloadUrl -OutFile $binaryPath -UseBasicParsing

Write-Host "Installed to $binaryPath" -ForegroundColor Green

# Migrate from legacy install location
Remove-LegacyInstall

# Add to user PATH if not already there
Add-ToPath $installDir

Write-Host ""
if ($isUpdate) {
    Write-Host "Done! The Associate has been updated to $($release.Version)." -ForegroundColor Green
} else {
    Write-Host "Done! Run 'assoc' to start The Associate." -ForegroundColor Green
}
