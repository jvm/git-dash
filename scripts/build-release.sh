#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-0.1.0}"
TARGETS=("x86_64-apple-darwin" "aarch64-apple-darwin")

echo "Building git-dash v${VERSION} for macOS..."
echo

for TARGET in "${TARGETS[@]}"; do
    echo "Building for ${TARGET}..."
    cargo build --release --target "${TARGET}"
    
    BINARY="target/${TARGET}/release/git-dash"
    if [ ! -f "${BINARY}" ]; then
        echo "Error: Binary not found at ${BINARY}"
        continue
    fi
    
    ARCHIVE="git-dash-v${VERSION}-${TARGET}.tar.gz"
    echo "Creating archive: ${ARCHIVE}"
    tar -czf "${ARCHIVE}" -C "target/${TARGET}/release" git-dash
    
    echo "Calculating SHA256..."
    shasum -a 256 "${ARCHIVE}"
    echo
done

echo "✓ All builds complete!"
echo
echo "Next steps:"
echo "1. Create a GitHub release at: https://github.com/jvm/git-dash/releases/new"
echo "2. Upload the .tar.gz files"
echo "3. Update homebrew-git-dash formula with the SHA256 checksums shown above"
