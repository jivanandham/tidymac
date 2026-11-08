#!/bin/bash
# Build TidyMac.app bundle and create .dmg installer
# Usage: ./scripts/build-dmg.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
APP_DIR="$PROJECT_DIR/TidyMac"
BUILD_DIR="$PROJECT_DIR/build"
APP_BUNDLE="$BUILD_DIR/TidyMac.app"
DMG_DIR="$BUILD_DIR/dmg"
DMG_OUTPUT="$BUILD_DIR/TidyMac.dmg"

echo "========================================="
echo "  TidyMac .dmg Builder"
echo "========================================="
echo ""

# Clean previous build
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# ─── Step 1: Build Rust FFI dylib ───────────────────────────────
echo "🔨 Step 1: Building Rust FFI library..."
cd "$PROJECT_DIR"

IS_UNIVERSAL=false
if command -v cargo >/dev/null 2>&1; then
    # Build for current architecture
    echo "   Building native $(uname -m) (library + CLI)..."
    cargo build --release

    OTHER_ARCH=""
    if [ "$(uname -m)" == "arm64" ]; then
        OTHER_ARCH="x86_64-apple-darwin"
    else
        OTHER_ARCH="aarch64-apple-darwin"
    fi

    echo "   Attempting universal build (trying $OTHER_ARCH)..."
    if cargo build --release --target "$OTHER_ARCH" 2>/dev/null; then
        echo "   Creating universal dylib and CLI with lipo..."
        mkdir -p "$PROJECT_DIR/target/universal"
        lipo -create \
            "$PROJECT_DIR/target/release/libtidymac.dylib" \
            "$PROJECT_DIR/target/$OTHER_ARCH/release/libtidymac.dylib" \
            -output "$PROJECT_DIR/target/universal/libtidymac.dylib"
        lipo -create \
            "$PROJECT_DIR/target/release/tidymac" \
            "$PROJECT_DIR/target/$OTHER_ARCH/release/tidymac" \
            -output "$PROJECT_DIR/target/universal/tidymac"
        DYLIB_SRC="$PROJECT_DIR/target/universal/libtidymac.dylib"
        CLI_SRC="$PROJECT_DIR/target/universal/tidymac"
        IS_UNIVERSAL=true
    else
        echo "   ⚠️  Could not build for $OTHER_ARCH. Defaulting to native build."
        DYLIB_SRC="$PROJECT_DIR/target/release/libtidymac.dylib"
        CLI_SRC="$PROJECT_DIR/target/release/tidymac"
        IS_UNIVERSAL=false
    fi
else
    echo "   ℹ️  cargo not found in PATH, using prebuilt dylib..."
    if [ -f "$PROJECT_DIR/target/release/libtidymac.dylib" ]; then
        DYLIB_SRC="$PROJECT_DIR/target/release/libtidymac.dylib"
    elif [ -f "$APP_DIR/Libraries/libtidymac.dylib" ]; then
        DYLIB_SRC="$APP_DIR/Libraries/libtidymac.dylib"
    else
        echo "❌ Rust dylib not found."
        exit 1
    fi
fi

if [ ! -f "$DYLIB_SRC" ]; then
    echo "❌ Rust dylib not found at $DYLIB_SRC"
    exit 1
fi
echo "   ✅ Rust dylib ready"

# Copy to Libraries for SPM build (install name @rpath so the app finds it in Frameworks)
cp "$DYLIB_SRC" "$APP_DIR/Libraries/libtidymac.dylib"
install_name_tool -id "@rpath/libtidymac.dylib" "$APP_DIR/Libraries/libtidymac.dylib"
cp "$PROJECT_DIR/ffi/tidymac.h" "$APP_DIR/Libraries/tidymac.h"

# ─── Step 2: Build Swift app ──────────────────────────────────
echo "🔨 Step 2: Building Swift app..."
cd "$APP_DIR"

if [ "$IS_UNIVERSAL" = true ]; then
    echo "   Building universal Swift app (arm64 + x86_64)..."
    if swift build -c release --arch arm64 --arch x86_64 2>/dev/null; then
        SWIFT_BIN="$APP_DIR/.build/apple/Products/Release/TidyMacApp"
        echo "   ✅ Universal Swift app built"
    else
        echo "   ⚠️  Universal Swift build failed. Falling back to native..."
        swift build -c release
        SWIFT_BIN="$APP_DIR/.build/release/TidyMacApp"
        echo "   ✅ Native Swift app built"
    fi
else
    echo "   Building native Swift app..."
    swift build -c release
    if [ -f "$APP_DIR/.build/out/Products/Release/TidyMacApp" ]; then
        SWIFT_BIN="$APP_DIR/.build/out/Products/Release/TidyMacApp"
    elif [ -f "$APP_DIR/.build/release/TidyMacApp" ]; then
        SWIFT_BIN="$APP_DIR/.build/release/TidyMacApp"
    fi
    echo "   ✅ Native Swift app built"
fi

if [ -z "$SWIFT_BIN" ] || [ ! -f "$SWIFT_BIN" ]; then
    if [ -f "$APP_DIR/.build/out/Products/Release/TidyMacApp" ]; then
        SWIFT_BIN="$APP_DIR/.build/out/Products/Release/TidyMacApp"
    fi
fi

if [ -z "$SWIFT_BIN" ] || [ ! -f "$SWIFT_BIN" ]; then
    echo "❌ Swift binary not found at $SWIFT_BIN"
    exit 1
fi

# ─── Step 3: Assemble .app bundle ──────────────────────────────
echo "📦 Step 3: Assembling TidyMac.app bundle..."

# Create bundle structure
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"
mkdir -p "$APP_BUNDLE/Contents/Frameworks"

# Copy executable
cp "$SWIFT_BIN" "$APP_BUNDLE/Contents/MacOS/TidyMacApp"

# Copy Info.plist
cp "$APP_DIR/Info.plist" "$APP_BUNDLE/Contents/Info.plist"

# Copy dylib into Frameworks
cp "$DYLIB_SRC" "$APP_BUNDLE/Contents/Frameworks/libtidymac.dylib"

# Bundle the CLI (used by scheduled reports: Contents/Resources/bin/tidymac)
if [ -n "${CLI_SRC:-}" ] && [ -f "$CLI_SRC" ]; then
    mkdir -p "$APP_BUNDLE/Contents/Resources/bin"
    cp "$CLI_SRC" "$APP_BUNDLE/Contents/Resources/bin/tidymac"
fi

# Fix the dylib's install name so it can be found via @rpath
install_name_tool -id "@rpath/libtidymac.dylib" "$APP_BUNDLE/Contents/Frameworks/libtidymac.dylib"

# Find every libtidymac reference in the executable and rewrite to @rpath
echo "   Fixing dylib references..."
CURRENT_DYLIB_REF=$(otool -L "$APP_BUNDLE/Contents/MacOS/TidyMacApp" | grep libtidymac | awk '{print $1}')
if [ -n "$CURRENT_DYLIB_REF" ] && [ "$CURRENT_DYLIB_REF" != "@rpath/libtidymac.dylib" ]; then
    echo "   Rewriting: '$CURRENT_DYLIB_REF' -> @rpath/libtidymac.dylib"
    install_name_tool -change "$CURRENT_DYLIB_REF" "@rpath/libtidymac.dylib" "$APP_BUNDLE/Contents/MacOS/TidyMacApp"
fi

# Ensure @executable_path/../Frameworks rpath exists
if ! otool -l "$APP_BUNDLE/Contents/MacOS/TidyMacApp" | grep -q "@executable_path/../Frameworks"; then
    install_name_tool -add_rpath "@executable_path/../Frameworks" "$APP_BUNDLE/Contents/MacOS/TidyMacApp"
fi

# Create PkgInfo
echo -n "APPL????" > "$APP_BUNDLE/Contents/PkgInfo"

# Generate app icon (simple text-based icns using sips if no icon exists)
ICON_DIR="$APP_DIR/Resources"
if [ -d "$ICON_DIR" ] && [ -f "$ICON_DIR/AppIcon.icns" ]; then
    cp "$ICON_DIR/AppIcon.icns" "$APP_BUNDLE/Contents/Resources/AppIcon.icns"
else
    echo "   No AppIcon.icns found — generating placeholder icon..."
    ICONSET_DIR="$BUILD_DIR/AppIcon.iconset"
    mkdir -p "$ICONSET_DIR"

    # Create a simple green icon using Python (HEREDOC avoids shell quoting issues)
    export ICONSET_PATH="$ICONSET_DIR"
    python3 - << 'EOF'
import struct, zlib, sys, os
path = os.environ.get('ICONSET_PATH', 'AppIcon.iconset')
def create_png(size, filepath):
    pixels = []
    center = size / 2
    radius = size * 0.42
    corner_r = size * 0.18
    for y in range(size):
        row = []
        for x in range(size):
            dx = abs(x - center)
            dy = abs(y - center)
            if dx <= radius - corner_r:
                inside = dy <= radius
            elif dy <= radius - corner_r:
                inside = dx <= radius
            else:
                cx = radius - corner_r
                cy = radius - corner_r
                dist = ((dx - cx)**2 + (dy - cy)**2)**0.5
                inside = dist <= corner_r
            if inside:
                t = (y / size)
                r, g, b = int(34 + t * 20), int(197 - t * 40), int(94 - t * 20)
                a = 255
            else:
                r, g, b, a = 0, 0, 0, 0
            row.extend([r, g, b, a])
        pixels.append(bytes(row))
    def make_png(width, height, rows):
        def chunk(ctype, data):
            c = ctype + data
            return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)
        raw = b''
        for row in rows: raw += b'\x00' + row
        return (b'\x89PNG\r\n\x1a\n' +
                chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0)) +
                chunk(b'IDAT', zlib.compress(raw)) +
                chunk(b'IEND', b''))
    with open(filepath, 'wb') as f:
        f.write(make_png(size, size, pixels))

sizes = [16, 32, 64, 128, 256, 512, 1024]
for s in sizes:
    create_png(s, os.path.join(path, f'icon_{s}x{s}.png'))
    if s <= 512:
        create_png(s*2, os.path.join(path, f'icon_{s}x{s}@2x.png'))
EOF

    if [ -d "$ICONSET_DIR" ] && [ -n "$(ls -A "$ICONSET_DIR" 2>/dev/null)" ]; then
        iconutil -c icns "$ICONSET_DIR" -o "$APP_BUNDLE/Contents/Resources/AppIcon.icns" 2>/dev/null || echo "   iconutil failed"
    fi
fi

echo "   ✅ App bundle assembled"

# ─── Step 4: Sign ─────────────────────────────────────────────
echo "🔏 Step 4: Code signing..."

# Use CODESIGN_IDENTITY if set; otherwise pick a real certificate from the
# keychain (Developer ID for distribution, Apple Development for personal use).
# A stable identity matters: macOS ties Full Disk Access to the signature, so
# an ad-hoc signed build loses its permission every time you rebuild.
SIGN_IDENTITY="${CODESIGN_IDENTITY:-}"
if [ -z "$SIGN_IDENTITY" ]; then
    SIGN_IDENTITY=$(security find-identity -v -p codesigning 2>/dev/null | grep -m1 "Developer ID Application" | sed -E 's/.*"(.*)"/\1/' || true)
fi
if [ -z "$SIGN_IDENTITY" ]; then
    SIGN_IDENTITY=$(security find-identity -v -p codesigning 2>/dev/null | grep -m1 "Apple Development" | sed -E 's/.*"(.*)"/\1/' || true)
fi
TIMESTAMP_FLAG="--timestamp"
if [ -z "$SIGN_IDENTITY" ]; then
    SIGN_IDENTITY="-"
    TIMESTAMP_FLAG=""
    echo ""
    echo "   ⚠️  No signing certificate found — using an ad-hoc signature."
    echo "   ⚠️  Full Disk Access will have to be re-granted after every rebuild."
    echo "   ⚠️  Fix: Xcode > Settings > Accounts > add your Apple ID > Manage Certificates > +"
    echo "       (a free 'Apple Development' certificate is enough for personal use)."
    echo ""
fi
if [ "$SIGN_IDENTITY" = "-" ]; then
    TIMESTAMP_FLAG=""
fi
echo "   Identity: $SIGN_IDENTITY"

codesign --remove-signature "$APP_BUNDLE/Contents/Frameworks/libtidymac.dylib" 2>/dev/null || true

echo "   Signing dylib..."
codesign --force --sign "$SIGN_IDENTITY" $TIMESTAMP_FLAG --options runtime \
    "$APP_BUNDLE/Contents/Frameworks/libtidymac.dylib"

if [ -f "$APP_BUNDLE/Contents/Resources/bin/tidymac" ]; then
    echo "   Signing bundled CLI..."
    codesign --force --sign "$SIGN_IDENTITY" $TIMESTAMP_FLAG --options runtime \
        --identifier "com.jeeva.TidyMac.cli" \
        "$APP_BUNDLE/Contents/Resources/bin/tidymac"
fi

echo "   Signing app bundle..."
codesign --force --sign "$SIGN_IDENTITY" $TIMESTAMP_FLAG \
    --entitlements "$APP_DIR/TidyMac.entitlements" \
    --options runtime \
    "$APP_BUNDLE"

echo "   Verifying signatures..."
codesign --verify --deep --strict "$APP_BUNDLE"
echo "   ✅ Code signed and verified"

# ─── Step 5: Create .dmg ───────────────────────────────────────
echo "💿 Step 5: Creating .dmg..."

rm -rf "$DMG_DIR"
mkdir -p "$DMG_DIR"
cp -R "$APP_BUNDLE" "$DMG_DIR/"
ln -s /Applications "$DMG_DIR/Applications"

rm -f "$DMG_OUTPUT"
hdiutil create \
    -volname "TidyMac" \
    -srcfolder "$DMG_DIR" \
    -ov \
    -format UDZO \
    -imagekey zlib-level=9 \
    "$DMG_OUTPUT"

rm -rf "$DMG_DIR"

# ─── Step 6 (optional): Notarize ────────────────────────────────
# Requires a Developer ID certificate and a stored notarytool profile:
#   xcrun notarytool store-credentials tidymac --apple-id … --team-id …
if [ -n "${NOTARY_PROFILE:-}" ] && [ "$SIGN_IDENTITY" != "-" ]; then
    echo "📨 Step 6: Notarizing..."
    codesign --force --sign "$SIGN_IDENTITY" --timestamp "$DMG_OUTPUT"
    xcrun notarytool submit "$DMG_OUTPUT" --keychain-profile "$NOTARY_PROFILE" --wait
    xcrun stapler staple "$DMG_OUTPUT"
    echo "   ✅ Notarized and stapled"
fi

echo ""
echo "========================================="
echo "  ✅ Build complete!"
echo "========================================="
echo ""
echo "  App:  $APP_BUNDLE"
echo "  DMG:  $DMG_OUTPUT"
echo ""
DMG_SIZE=$(du -h "$DMG_OUTPUT" | awk '{print $1}')
echo "  DMG size: $DMG_SIZE"
echo ""
