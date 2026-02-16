#!/bin/bash
# TidyMac Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/jeevakrishnasamy/tidymac/main/scripts/install.sh | bash

set -e

REPO="jeevakrishnasamy/tidymac"
INSTALL_DIR="/usr/local/bin"
VERSION="latest"

echo "🧹 Installing TidyMac..."

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    arm64|aarch64)
        TARGET="aarch64-apple-darwin"
        ;;
    x86_64)
        TARGET="x86_64-apple-darwin"
        ;;
    *)
        echo "❌ Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Check OS
OS=$(uname -s)
if [ "$OS" != "Darwin" ]; then
    echo "❌ TidyMac only supports macOS. Detected: $OS"
    exit 1
fi

echo "  Architecture: $ARCH ($TARGET)"

# Get latest release URL
if [ "$VERSION" = "latest" ]; then
    DOWNLOAD_URL=$(curl -sL "https://api.github.com/repos/${REPO}/releases/latest" | \
        grep "browser_download_url.*${TARGET}" | \
        head -1 | \
        cut -d '"' -f 4)
else
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/tidymac-${TARGET}.tar.gz"
fi

if [ -z "$DOWNLOAD_URL" ]; then
    echo "❌ Could not find release for $TARGET"
    echo "  Try building from source: cargo install --git https://github.com/${REPO}"
    exit 1
fi

echo "  Downloading from: $DOWNLOAD_URL"

# Download and install
TMPDIR=$(mktemp -d)
curl -sL "$DOWNLOAD_URL" -o "$TMPDIR/tidymac.tar.gz"
cd "$TMPDIR"
tar -xzf tidymac.tar.gz

# Install binary
if [ -w "$INSTALL_DIR" ]; then
    mv tidymac "$INSTALL_DIR/tidymac"
else
    echo "  Requires sudo for $INSTALL_DIR..."
    sudo mv tidymac "$INSTALL_DIR/tidymac"
fi

# Cleanup
rm -rf "$TMPDIR"

# Initialize
tidymac config init 2>/dev/null || true

echo ""
echo "✅ TidyMac installed to $INSTALL_DIR/tidymac"
echo ""
echo "  Get started:"
echo "    tidymac scan --profile developer"
echo "    tidymac --help"
echo ""
