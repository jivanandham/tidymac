#!/bin/bash
# TidyMac Release Script
# Usage: ./scripts/release.sh 1.0.0

set -e

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    echo "Usage: ./scripts/release.sh <version>"
    echo "Example: ./scripts/release.sh 1.0.0"
    exit 1
fi

echo "🧹 TidyMac Release v${VERSION}"
echo "─────────────────────────────────"

# 1. Update version in Cargo.toml
echo "📝 Updating Cargo.toml version to ${VERSION}..."
sed -i '' "s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml

# 2. Build for both architectures
echo "🔨 Building for arm64..."
cargo build --release --target aarch64-apple-darwin

echo "🔨 Building for x86_64..."
cargo build --release --target x86_64-apple-darwin

# 3. Create universal binary
echo "🔗 Creating universal binary..."
lipo -create \
    target/aarch64-apple-darwin/release/tidymac \
    target/x86_64-apple-darwin/release/tidymac \
    -output target/tidymac-universal
file target/tidymac-universal

# 4. Package binaries
echo "📦 Packaging..."
mkdir -p dist

cd target/aarch64-apple-darwin/release
tar -czf ../../../dist/tidymac-aarch64-apple-darwin.tar.gz tidymac
cd ../../..

cd target/x86_64-apple-darwin/release
tar -czf ../../../dist/tidymac-x86_64-apple-darwin.tar.gz tidymac
cd ../../..

cp target/tidymac-universal dist/tidymac
cd dist
tar -czf tidymac-universal-apple-darwin.tar.gz tidymac
rm tidymac
cd ..

# 5. Generate checksums
echo "🔐 Generating checksums..."
cd dist
shasum -a 256 *.tar.gz > checksums.txt
cat checksums.txt
cd ..

# 6. Generate shell completions
echo "📋 Generating shell completions..."
target/aarch64-apple-darwin/release/tidymac completions bash > dist/tidymac.bash
target/aarch64-apple-darwin/release/tidymac completions zsh > dist/_tidymac
target/aarch64-apple-darwin/release/tidymac completions fish > dist/tidymac.fish

# 7. Instructions for completing the release
echo ""
echo "─────────────────────────────────"
echo "✅ Build complete! Files in dist/"
echo ""
echo "Next steps:"
echo ""
echo "1. Commit and tag:"
echo "   git add -A"
echo "   git commit -m 'release: v${VERSION}'"
echo "   git tag v${VERSION}"
echo "   git push origin main --tags"
echo ""
echo "2. The GitHub Release workflow will automatically:"
echo "   - Build arm64 and x86_64 binaries"
echo "   - Create a universal binary"
echo "   - Create a GitHub Release with all artifacts"
echo ""
echo "3. After the release is published, update the Homebrew formula:"
echo "   - Get the SHA256 from the release assets"
echo "   - Update homebrew-tidymac/Formula/tidymac.rb with new SHA256 values"
echo "   - Push to the homebrew-tidymac repository"
echo ""
echo "4. Users can then install with:"
echo "   brew tap jeevakrishnasamy/tidymac"
echo "   brew install tidymac"
echo ""
