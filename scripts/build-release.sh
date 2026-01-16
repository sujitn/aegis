#!/usr/bin/env bash
# Build release binaries and generate checksums
# Usage: ./scripts/build-release.sh [--all-targets]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
RELEASE_DIR="$PROJECT_ROOT/release"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# Detect platform
detect_platform() {
    case "$(uname -s)" in
        Darwin)
            if [ "$(uname -m)" = "arm64" ]; then
                echo "aarch64-apple-darwin"
            else
                echo "x86_64-apple-darwin"
            fi
            ;;
        Linux)
            echo "x86_64-unknown-linux-gnu"
            ;;
        MINGW*|CYGWIN*|MSYS*)
            echo "x86_64-pc-windows-msvc"
            ;;
        *)
            error "Unsupported platform: $(uname -s)"
            ;;
    esac
}

# Build for a specific target
build_target() {
    local target="$1"
    info "Building for target: $target"

    # Check if we need to add the target
    if ! rustup target list --installed | grep -q "$target"; then
        info "Installing target: $target"
        rustup target add "$target"
    fi

    cargo build --release --target "$target"

    local binary_name="aegis"
    if [[ "$target" == *"windows"* ]]; then
        binary_name="aegis.exe"
    fi

    local output_dir="$RELEASE_DIR/$target"
    mkdir -p "$output_dir"
    cp "$PROJECT_ROOT/target/$target/release/$binary_name" "$output_dir/"

    info "Binary built: $output_dir/$binary_name"
}

# Build extension
build_extension() {
    info "Building browser extension..."
    cd "$PROJECT_ROOT/extension"

    if [ ! -d "node_modules" ]; then
        npm install
    fi

    npm run build

    # Create extension zip
    mkdir -p "$RELEASE_DIR"
    local version
    version=$(grep '"version"' manifest.json | head -1 | cut -d'"' -f4)
    local zip_name="aegis-extension-$version.zip"

    rm -f "$RELEASE_DIR/$zip_name"
    zip -r "$RELEASE_DIR/$zip_name" manifest.json popup.html overlay.css icons/ dist/

    info "Extension built: $RELEASE_DIR/$zip_name"
    cd "$PROJECT_ROOT"
}

# Generate checksums
generate_checksums() {
    info "Generating checksums..."
    cd "$RELEASE_DIR"

    # Find all release files
    find . -maxdepth 2 -type f \( -name "aegis" -o -name "aegis.exe" -o -name "*.zip" -o -name "*.tar.gz" \) | while read -r file; do
        if command -v sha256sum &> /dev/null; then
            sha256sum "$file" >> SHA256SUMS.txt
        elif command -v shasum &> /dev/null; then
            shasum -a 256 "$file" >> SHA256SUMS.txt
        fi
    done

    if [ -f SHA256SUMS.txt ]; then
        info "Checksums written to: $RELEASE_DIR/SHA256SUMS.txt"
        cat SHA256SUMS.txt
    fi

    cd "$PROJECT_ROOT"
}

# Main
main() {
    cd "$PROJECT_ROOT"

    # Clean release directory
    rm -rf "$RELEASE_DIR"
    mkdir -p "$RELEASE_DIR"

    if [ "${1:-}" = "--all-targets" ]; then
        # Build for all supported targets (requires cross-compilation setup)
        warn "Building all targets requires cross-compilation toolchains"
        for target in x86_64-apple-darwin aarch64-apple-darwin x86_64-pc-windows-msvc x86_64-unknown-linux-gnu; do
            build_target "$target" || warn "Failed to build $target (cross-compilation may not be set up)"
        done
    else
        # Build for current platform only
        local target
        target=$(detect_platform)
        build_target "$target"
    fi

    build_extension
    generate_checksums

    info "Release build complete!"
    info "Artifacts in: $RELEASE_DIR"
}

main "$@"
