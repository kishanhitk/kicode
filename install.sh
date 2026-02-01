#!/bin/sh
# Kicode installer script
# Usage: curl -fsSL https://raw.githubusercontent.com/kishanhitk/kicode/master/install.sh | sh

set -e

REPO="kishanhitk/kicode"
BINARY_NAME="kicode"
INSTALL_DIR="${KICODE_INSTALL_DIR:-$HOME/.local/bin}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() {
    printf "${BLUE}info${NC}: %s\n" "$1"
}

success() {
    printf "${GREEN}success${NC}: %s\n" "$1"
}

warn() {
    printf "${YELLOW}warning${NC}: %s\n" "$1"
}

error() {
    printf "${RED}error${NC}: %s\n" "$1" >&2
    exit 1
}

# Check platform
check_platform() {
    OS=$(uname -s)
    ARCH=$(uname -m)

    if [ "$OS" != "Darwin" ]; then
        error "Currently only macOS is supported. Found: $OS"
    fi

    if [ "$ARCH" != "arm64" ]; then
        error "Currently only Apple Silicon (M1/M2/M3) is supported. Found: $ARCH"
    fi

    echo "aarch64-apple-darwin"
}

# Get latest release version from GitHub
get_latest_version() {
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" |
        grep '"tag_name":' |
        sed -E 's/.*"([^"]+)".*/\1/'
}

main() {
    echo ""
    printf "${GREEN}Kicode Installer${NC}\n"
    echo "================"
    echo ""

    # Check for required commands
    if ! command -v curl >/dev/null 2>&1; then
        error "curl is required but not installed"
    fi

    if ! command -v tar >/dev/null 2>&1; then
        error "tar is required but not installed"
    fi

    # Check platform
    TARGET=$(check_platform)
    info "Detected platform: ${TARGET}"

    # Get version
    VERSION="${KICODE_VERSION:-$(get_latest_version)}"
    if [ -z "$VERSION" ]; then
        error "Could not determine latest version. Please check your internet connection."
    fi

    info "Installing version: ${VERSION}"

    # Construct download URL
    ARCHIVE_NAME="${BINARY_NAME}-${TARGET}.tar.gz"
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE_NAME}"

    # Create temp directory
    TMP_DIR=$(mktemp -d)
    trap 'rm -rf "$TMP_DIR"' EXIT

    # Download
    info "Downloading from ${DOWNLOAD_URL}"
    if ! curl -fsSL "$DOWNLOAD_URL" -o "${TMP_DIR}/${ARCHIVE_NAME}"; then
        error "Failed to download ${ARCHIVE_NAME}. Please check if the release exists."
    fi

    # Extract
    info "Extracting archive..."
    tar -xzf "${TMP_DIR}/${ARCHIVE_NAME}" -C "$TMP_DIR"

    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"

    # Install
    info "Installing to ${INSTALL_DIR}/${BINARY_NAME}"
    mv "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

    echo ""
    success "Kicode ${VERSION} installed successfully!"
    echo ""

    # Check if install dir is in PATH
    case ":$PATH:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            warn "${INSTALL_DIR} is not in your PATH"
            echo ""
            echo "Add it to your shell configuration:"
            echo ""
            echo "  For zsh (~/.zshrc):"
            echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
            echo ""
            echo "Then restart your shell or run: source ~/.zshrc"
            ;;
    esac

    echo ""
    echo "Get started by running:"
    echo "  ${BINARY_NAME} setup"
    echo ""
}

main "$@"
