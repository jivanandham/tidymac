#!/bin/bash
# Build TidyMac Rust library for SwiftUI integration
# Produces a universal (arm64 + x86_64) dylib

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="$PROJECT_DIR/TidyMacApp/Libraries"

echo "🔨 Building TidyMac Rust library..."

cd "$PROJECT_DIR"

# Build for current architecture (release)
cargo build --release --lib

# Determine architecture
ARCH=$(uname -m)
if [ "$ARCH" = "arm64" ]; then
    DYLIB_PATH="$PROJECT_DIR/target/release/libtidymac.dylib"
else
    DYLIB_PATH="$PROJECT_DIR/target/release/libtidymac.dylib"
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Copy dylib
cp "$DYLIB_PATH" "$OUTPUT_DIR/libtidymac.dylib"

# Copy header
cp "$PROJECT_DIR/ffi/tidymac.h" "$OUTPUT_DIR/tidymac.h"

# Create module.modulemap for Swift
cat > "$OUTPUT_DIR/module.modulemap" << 'EOF'
module TidyMacFFI {
    header "tidymac.h"
    link "tidymac"
    export *
}
EOF

echo "✅ Built successfully!"
echo "   Library: $OUTPUT_DIR/libtidymac.dylib"
echo "   Header:  $OUTPUT_DIR/tidymac.h"
echo "   Module:  $OUTPUT_DIR/module.modulemap"
