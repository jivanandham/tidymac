#!/bin/bash
set -e

# TidyMac Release Script
# Usage: ./release.sh [version]

VERSION=${1:-"1.0.0"}
APP_NAME="TidyMac"
BUNDLE_ID="com.tidymac.app"
DEVELOPER_ID="Developer ID Application: Jeeva Krishnasamy (XXXXXXXXXX)"

echo "🚀 Starting release process for TidyMac v$VERSION..."

# 1. Build Rust Core (Universal Binary)
echo "📦 Building Rust core..."
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
lipo -create \
  target/x86_64-apple-darwin/release/libtidymac.dylib \
  target/aarch64-apple-darwin/release/libtidymac.dylib \
  -output target/release/libtidymac.dylib

# 2. Build SwiftUI App
echo "🏗️ Building SwiftUI app..."
xcodebuild -project TidyMac.xcodeproj \
           -scheme TidyMac \
           -configuration Release \
           -archivePath build/TidyMac.xcarchive \
           archive

# 3. Export and Sign
echo "🖋️ Signing app bundle..."
xcodebuild -exportArchive \
           -archivePath build/TidyMac.xcarchive \
           -exportOptionsPlist exportOptions.plist \
           -exportPath build/dist

# 4. Notarize
echo "🛡️ Notarizing..."
# xcrun notarytool submit build/dist/TidyMac.dmg --apple-id ... --password ... --team-id ... --wait

# 5. Generate Sparkle Appcast
echo "✨ Generating Sparkle appcast..."
# bin/generate_appcast build/dist

echo "✅ Done! Release ready in build/dist/"
