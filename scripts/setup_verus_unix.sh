#!/bin/bash
set -e

VERSION="0.2026.03.10.13c14a1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="$SCRIPT_DIR/../verus"

OS=$(uname -s)
ARCH=$(uname -m)

if [ "$OS" = "Darwin" ]; then
    if [ "$ARCH" = "arm64" ]; then
        ZIP_NAME="verus-${VERSION}-arm64-macos.zip"
        TOOLCHAIN="1.93.1-aarch64-apple-darwin"
    else
        ZIP_NAME="verus-${VERSION}-x86-macos.zip"
        TOOLCHAIN="1.93.1-x86_64-apple-darwin"
    fi
elif [ "$OS" = "Linux" ]; then
    ZIP_NAME="verus-${VERSION}-x86-linux.zip"
    TOOLCHAIN="1.93.1-x86_64-unknown-linux-gnu"
else
    echo "Unsupported OS: $OS. Use setup_verus_windows.ps1 on Windows."
    exit 1
fi

URL="https://github.com/verus-lang/verus/releases/download/release/${VERSION}/${ZIP_NAME}"

echo "==> Downloading Verus ${VERSION} for ${OS}/${ARCH}..."
curl -L "$URL" -o /tmp/verus.zip

echo "==> Extracting to $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"
TMP_EXTRACT=$(mktemp -d)
unzip -o /tmp/verus.zip -d "$TMP_EXTRACT"
cp -r "$TMP_EXTRACT"/verus-*/. "$INSTALL_DIR/"
rm -rf /tmp/verus.zip "$TMP_EXTRACT"

if [ "$OS" = "Darwin" ]; then
    echo "==> Removing Gatekeeper quarantine..."
    xattr -rd com.apple.quarantine "$INSTALL_DIR"
fi

echo "==> Installing Rust toolchain ${TOOLCHAIN}..."
rustup toolchain install "$TOOLCHAIN"

echo "==> Verifying..."
"$INSTALL_DIR/verus" --version

echo ""
echo "Done. To verify a file:"
echo "  $INSTALL_DIR/verus <path/to/file.rs>"
