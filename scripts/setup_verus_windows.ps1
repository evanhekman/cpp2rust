$ErrorActionPreference = "Stop"

$VERSION = "0.2026.03.10.13c14a1"
$TOOLCHAIN = "1.93.1-x86_64-pc-windows-msvc"
$ZIP_NAME = "verus-$VERSION-x86-win.zip"
$URL = "https://github.com/verus-lang/verus/releases/download/release/$VERSION/$ZIP_NAME"
$INSTALL_DIR = Join-Path $PSScriptRoot "..\verus"

Write-Host "==> Downloading Verus $VERSION..."
Invoke-WebRequest -Uri $URL -OutFile "$env:TEMP\verus.zip"

Write-Host "==> Extracting to $INSTALL_DIR..."
New-Item -ItemType Directory -Force -Path $INSTALL_DIR | Out-Null
$TMP_EXTRACT = Join-Path $env:TEMP "verus_extract"
Expand-Archive -Path "$env:TEMP\verus.zip" -DestinationPath $TMP_EXTRACT -Force
$INNER = Get-ChildItem $TMP_EXTRACT -Directory | Select-Object -First 1
Copy-Item -Path "$($INNER.FullName)\*" -Destination $INSTALL_DIR -Recurse -Force
Remove-Item "$env:TEMP\verus.zip", $TMP_EXTRACT -Recurse -Force

Write-Host "==> Installing Rust toolchain $TOOLCHAIN..."
rustup toolchain install $TOOLCHAIN

Write-Host "==> Verifying..."
& "$INSTALL_DIR\verus.exe" --version

Write-Host ""
Write-Host "Done. To verify a file:"
Write-Host "  $INSTALL_DIR\verus.exe <path\to\file.rs>"
