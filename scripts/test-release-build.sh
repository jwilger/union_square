#!/usr/bin/env bash
# Test release build locally

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "Testing release build process locally..."
echo "Project root: ${PROJECT_ROOT}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[+]${NC} $1"
}

print_error() {
    echo -e "${RED}[!]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[*]${NC} $1"
}

# Check if we're in the project root
if [[ ! -f "${PROJECT_ROOT}/Cargo.toml" ]]; then
    print_error "This script must be run from a Rust project directory with Cargo.toml"
    exit 1
fi

# Extract project name from Cargo.toml
PROJECT_NAME=$(grep -E '^name = ' "${PROJECT_ROOT}/Cargo.toml" | head -1 | sed 's/name = "\(.*\)"/\1/')
if [[ -z "${PROJECT_NAME}" ]]; then
    print_error "Could not extract project name from Cargo.toml"
    exit 1
fi

cd "${PROJECT_ROOT}"

# Test native build
print_status "Testing native release build..."
cargo build --release --bin union_square

if [[ -f "target/release/union_square" ]] || [[ -f "target/release/union_square.exe" ]]; then
    print_status "Native build successful!"
else
    print_error "Native build failed!"
    exit 1
fi

# Test Docker build
if command -v docker &> /dev/null; then
    print_status "Testing Docker build..."
    docker build -t union_square:test .
    print_status "Docker build successful!"
else
    print_warning "Docker not found, skipping Docker build test"
fi

# Test archive creation
print_status "Testing archive creation..."
cd target/release

if [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
    # Windows
    if command -v 7z &> /dev/null; then
        7z a ../../test-release.zip union_square.exe
        print_status "Windows archive created successfully!"
    else
        print_warning "7z not found, skipping Windows archive test"
    fi
else
    # Unix-like
    tar czf ../../test-release.tar.gz union_square
    cd ../..
    sha256sum test-release.tar.gz > test-release.tar.gz.sha256
    print_status "Unix archive created successfully!"
    print_status "Checksum: $(cat test-release.tar.gz.sha256)"
fi

cd "${PROJECT_ROOT}"

# Cleanup
print_status "Cleaning up test artifacts..."
rm -f test-release.tar.gz test-release.tar.gz.sha256 test-release.zip

print_status "Release build test completed successfully!"
print_status ""
print_status "To test cross-compilation, you can install 'cross' and run:"
print_status "  cargo install cross"
print_status "  cross build --release --target aarch64-unknown-linux-gnu"
