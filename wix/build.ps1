# Build the Meleys MSI installer with the WiX toolset.
#
# Requirements:
#   - WiX v3 toolset on PATH (candle.exe + light.exe).
#     Install via:  winget install WiX.WiXToolset
#   - A release build already produced at ..\target\release\meleys.exe
#     (run:  cargo build --release  in the project root first)
#
# Output: meleys.msi in this directory.

$ErrorActionPreference = "Stop"

$root   = Resolve-Path (Join-Path $PSScriptRoot "..")
$wixDir = $PSScriptRoot
$binDir = Join-Path $root "target\release"

$assetDir = Join-Path $root "assets"

if (-not (Test-Path (Join-Path $binDir "meleys.exe"))) {
    Write-Error "meleys.exe not found in $binDir. Run 'cargo build --release' first."
    exit 1
}

# Read version from Cargo.toml ([package] version = "x.y.z")
$cargo = Get-Content (Join-Path $root "Cargo.toml") -Raw
if ($cargo -match '(?m)^\s*version\s*=\s*"([^"]+)"') {
    $version = $Matches[1]
} else {
    $version = "0.1.0"
}

$candle = Get-Command candle.exe -ErrorAction SilentlyContinue
$light  = Get-Command light.exe  -ErrorAction SilentlyContinue
if (-not $candle -or -not $light) {
    Write-Error "WiX toolset not found on PATH. Install it with: winget install WiX.WiXToolset"
    exit 1
}

Write-Host "Building Meleys installer v$version ..."

& candle.exe "$wixDir\meleys.wxs" `
    "-dProductVersion=$version" `
    "-dBinDir=$binDir" `
    "-dWixDir=$wixDir" `
    "-dAssetDir=$assetDir" `
    -out "$wixDir\meleys.wixobj"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

& light.exe "$wixDir\meleys.wixobj" `
    -ext WixUIExtension `
    -out "$wixDir\meleys.msi"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Done: $wixDir\meleys.msi"
