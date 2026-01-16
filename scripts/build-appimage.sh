#!/usr/bin/env bash
# Build AppImage for Aegis
# Usage: ./scripts/build-appimage.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
APP_DIR="$PROJECT_ROOT/crates/aegis-app"
BUILD_DIR="$PROJECT_ROOT/target/appimage"
APPDIR="$BUILD_DIR/Aegis.AppDir"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# Check for required tools
check_dependencies() {
    if ! command -v appimagetool &> /dev/null; then
        info "Downloading appimagetool..."
        wget -q "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage" \
            -O /tmp/appimagetool
        chmod +x /tmp/appimagetool
        APPIMAGETOOL="/tmp/appimagetool"
    else
        APPIMAGETOOL="appimagetool"
    fi
}

# Build the binary
build_binary() {
    info "Building release binary..."
    cd "$PROJECT_ROOT"
    cargo build --release --package aegis-app
}

# Create AppDir structure
create_appdir() {
    info "Creating AppDir structure..."

    rm -rf "$APPDIR"
    mkdir -p "$APPDIR/usr/bin"
    mkdir -p "$APPDIR/usr/share/applications"
    mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"
    mkdir -p "$APPDIR/usr/share/metainfo"

    # Copy binary
    cp "$PROJECT_ROOT/target/release/aegis" "$APPDIR/usr/bin/"

    # Copy desktop entry
    cp "$APP_DIR/appimage/aegis.desktop" "$APPDIR/"
    cp "$APP_DIR/appimage/aegis.desktop" "$APPDIR/usr/share/applications/"

    # Copy AppStream metadata
    cp "$APP_DIR/appimage/aegis.appdata.xml" "$APPDIR/usr/share/metainfo/"

    # Copy icon (use placeholder if not exists)
    if [ -f "$APP_DIR/assets/icons/icon-256.png" ]; then
        cp "$APP_DIR/assets/icons/icon-256.png" "$APPDIR/aegis.png"
        cp "$APP_DIR/assets/icons/icon-256.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/aegis.png"
    else
        info "Warning: icon-256.png not found, creating placeholder..."
        # Create a simple placeholder icon using ImageMagick if available
        if command -v convert &> /dev/null; then
            convert -size 256x256 xc:#4A90D9 -fill white -gravity center \
                -pointsize 120 -annotate 0 'A' "$APPDIR/aegis.png"
            cp "$APPDIR/aegis.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/aegis.png"
        else
            error "No icon found and ImageMagick not available to create placeholder"
        fi
    fi

    # Create AppRun
    cat > "$APPDIR/AppRun" << 'EOF'
#!/bin/bash
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
export PATH="${HERE}/usr/bin:${PATH}"
exec "${HERE}/usr/bin/aegis" "$@"
EOF
    chmod +x "$APPDIR/AppRun"
}

# Build the AppImage
build_appimage() {
    info "Building AppImage..."

    cd "$BUILD_DIR"

    # Get version
    VERSION=$(grep '^version' "$PROJECT_ROOT/Cargo.toml" | head -1 | cut -d'"' -f2 || echo "0.1.0")

    # Build AppImage
    ARCH=x86_64 "$APPIMAGETOOL" "$APPDIR" "Aegis-${VERSION}-x86_64.AppImage"

    info "AppImage created: $BUILD_DIR/Aegis-${VERSION}-x86_64.AppImage"
}

# Main
main() {
    info "Starting AppImage build..."

    check_dependencies
    build_binary
    create_appdir
    build_appimage

    info "Done!"
}

main "$@"
