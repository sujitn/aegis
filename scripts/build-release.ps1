# Build release binaries and generate checksums (Windows)
# Usage: .\scripts\build-release.ps1

param(
    [switch]$AllTargets
)

$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$ReleaseDir = Join-Path $ProjectRoot "release"

function Write-Info { param($Message) Write-Host "[INFO] $Message" -ForegroundColor Green }
function Write-Warn { param($Message) Write-Host "[WARN] $Message" -ForegroundColor Yellow }
function Write-Err { param($Message) Write-Host "[ERROR] $Message" -ForegroundColor Red; exit 1 }

function Build-Target {
    param($Target)

    Write-Info "Building for target: $Target"

    # Check if target is installed
    $installed = rustup target list --installed | Select-String $Target
    if (-not $installed) {
        Write-Info "Installing target: $Target"
        rustup target add $Target
    }

    cargo build --release --target $Target
    if ($LASTEXITCODE -ne 0) {
        Write-Err "Build failed for $Target"
    }

    $binaryName = if ($Target -like "*windows*") { "aegis.exe" } else { "aegis" }
    $outputDir = Join-Path $ReleaseDir $Target

    New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
    Copy-Item (Join-Path $ProjectRoot "target\$Target\release\$binaryName") $outputDir

    Write-Info "Binary built: $outputDir\$binaryName"
}

function Build-Extension {
    Write-Info "Building browser extension..."

    Push-Location (Join-Path $ProjectRoot "extension")
    try {
        if (-not (Test-Path "node_modules")) {
            npm install
        }

        npm run build
        if ($LASTEXITCODE -ne 0) {
            Write-Err "Extension build failed"
        }

        # Get version from manifest
        $manifest = Get-Content "manifest.json" | ConvertFrom-Json
        $version = $manifest.version
        $zipName = "aegis-extension-$version.zip"
        $zipPath = Join-Path $ReleaseDir $zipName

        # Create zip (requires PowerShell 5.0+)
        $filesToZip = @(
            "manifest.json",
            "popup.html",
            "overlay.css",
            "icons",
            "dist"
        )

        if (Test-Path $zipPath) { Remove-Item $zipPath }
        Compress-Archive -Path $filesToZip -DestinationPath $zipPath

        Write-Info "Extension built: $zipPath"
    }
    finally {
        Pop-Location
    }
}

function Generate-Checksums {
    Write-Info "Generating checksums..."

    Push-Location $ReleaseDir
    try {
        $checksumFile = "SHA256SUMS.txt"
        if (Test-Path $checksumFile) { Remove-Item $checksumFile }

        # Find all release files
        $files = Get-ChildItem -Recurse -File | Where-Object {
            $_.Name -match "^aegis(\.exe)?$" -or $_.Name -match "\.(zip|tar\.gz)$"
        }

        foreach ($file in $files) {
            $hash = Get-FileHash -Path $file.FullName -Algorithm SHA256
            $relativePath = $file.FullName.Replace($ReleaseDir, ".").Replace("\", "/")
            "$($hash.Hash.ToLower())  $relativePath" | Out-File -Append -Encoding UTF8 $checksumFile
        }

        if (Test-Path $checksumFile) {
            Write-Info "Checksums written to: $ReleaseDir\$checksumFile"
            Get-Content $checksumFile
        }
    }
    finally {
        Pop-Location
    }
}

# Main
Push-Location $ProjectRoot
try {
    # Clean release directory
    if (Test-Path $ReleaseDir) { Remove-Item -Recurse -Force $ReleaseDir }
    New-Item -ItemType Directory -Force -Path $ReleaseDir | Out-Null

    if ($AllTargets) {
        Write-Warn "Building all targets requires cross-compilation toolchains"
        $targets = @(
            "x86_64-pc-windows-msvc",
            "x86_64-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin"
        )
        foreach ($target in $targets) {
            try {
                Build-Target $target
            }
            catch {
                Write-Warn "Failed to build $target (cross-compilation may not be set up)"
            }
        }
    }
    else {
        Build-Target "x86_64-pc-windows-msvc"
    }

    Build-Extension
    Generate-Checksums

    Write-Info "Release build complete!"
    Write-Info "Artifacts in: $ReleaseDir"
}
finally {
    Pop-Location
}
