#!/bin/sh
# Installation script for bu CLI
# To make executable: chmod +x install.sh
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/anthropics/bu/main/cli-impl/install.sh | sh
#   or
#   ./install.sh --install-dir /usr/local/bin

set -e

# Configuration
REPO="${BU_REPO:-anthropics/bu}"
INSTALL_DIR="${BU_INSTALL_DIR:-$HOME/.bu/bin}"
BINARY_NAME="bu"

# Colors (disabled if not a tty)
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    NC=''
fi

# Logging functions
log_info() {
    printf "${BLUE}info${NC}: %s\n" "$1"
}

log_success() {
    printf "${GREEN}success${NC}: %s\n" "$1"
}

log_warn() {
    printf "${YELLOW}warning${NC}: %s\n" "$1"
}

log_error() {
    printf "${RED}error${NC}: %s\n" "$1" >&2
}

# Print help
print_help() {
    cat << EOF
bu installer

Usage: install.sh [OPTIONS]

Options:
    -h, --help              Print this help message
    -d, --install-dir DIR   Install to DIR (default: ~/.bu/bin)
    -v, --version VERSION   Install specific version (default: latest)
    --no-path-update        Don't update shell config with PATH
    --repo OWNER/REPO       Use different GitHub repository

Environment variables:
    BU_REPO         GitHub repository (default: anthropics/bu)
    BU_INSTALL_DIR  Installation directory (default: ~/.bu/bin)

Examples:
    # Install latest version
    curl -fsSL https://raw.githubusercontent.com/anthropics/bu/main/cli-impl/install.sh | sh

    # Install specific version
    ./install.sh --version v1.0.0

    # Install to custom directory
    ./install.sh --install-dir /usr/local/bin
EOF
}

# Detect platform (Linux/Darwin)
detect_platform() {
    platform=$(uname -s | tr '[:upper:]' '[:lower:]')
    case "$platform" in
        linux)
            echo "unknown-linux-musl"
            ;;
        darwin)
            echo "apple-darwin"
            ;;
        *)
            log_error "Unsupported platform: $platform"
            exit 1
            ;;
    esac
}

# Detect architecture (x86_64/aarch64)
detect_arch() {
    arch=$(uname -m)
    case "$arch" in
        x86_64|amd64)
            echo "x86_64"
            ;;
        aarch64|arm64)
            echo "aarch64"
            ;;
        *)
            log_error "Unsupported architecture: $arch"
            exit 1
            ;;
    esac
}

# Check for required commands
check_dependencies() {
    if command -v curl > /dev/null 2>&1; then
        DOWNLOADER="curl"
    elif command -v wget > /dev/null 2>&1; then
        DOWNLOADER="wget"
    else
        log_error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi

    if ! command -v tar > /dev/null 2>&1; then
        log_error "tar not found. Please install tar."
        exit 1
    fi
}

# Download a file
download_file() {
    url="$1"
    output="$2"

    log_info "Downloading from $url"

    if [ "$DOWNLOADER" = "curl" ]; then
        curl -fsSL "$url" -o "$output"
    else
        wget -q "$url" -O "$output"
    fi
}

# Get latest release version from GitHub
get_latest_version() {
    url="https://api.github.com/repos/$REPO/releases/latest"

    if [ "$DOWNLOADER" = "curl" ]; then
        version=$(curl -fsSL "$url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    else
        version=$(wget -qO- "$url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    fi

    if [ -z "$version" ]; then
        log_error "Failed to get latest version. Please specify a version with --version"
        exit 1
    fi

    echo "$version"
}

# Verify checksum if available
verify_checksum() {
    binary_path="$1"
    checksum_path="$2"

    if [ ! -f "$checksum_path" ]; then
        log_warn "Checksum file not found, skipping verification"
        return 0
    fi

    expected=$(cat "$checksum_path" | awk '{print $1}')

    if command -v sha256sum > /dev/null 2>&1; then
        actual=$(sha256sum "$binary_path" | awk '{print $1}')
    elif command -v shasum > /dev/null 2>&1; then
        actual=$(shasum -a 256 "$binary_path" | awk '{print $1}')
    else
        log_warn "No SHA256 tool found, skipping checksum verification"
        return 0
    fi

    if [ "$expected" != "$actual" ]; then
        log_error "Checksum verification failed!"
        log_error "Expected: $expected"
        log_error "Actual:   $actual"
        exit 1
    fi

    log_success "Checksum verified"
}

# Update shell configuration to add to PATH
update_shell_config() {
    install_dir="$1"
    path_line="export PATH=\"$install_dir:\$PATH\""

    # Check if already in PATH
    case ":$PATH:" in
        *":$install_dir:"*)
            log_info "Directory already in PATH"
            return 0
            ;;
    esac

    updated=0

    # Update bashrc
    if [ -f "$HOME/.bashrc" ]; then
        if ! grep -q "$install_dir" "$HOME/.bashrc" 2>/dev/null; then
            echo "" >> "$HOME/.bashrc"
            echo "# Added by bu installer" >> "$HOME/.bashrc"
            echo "$path_line" >> "$HOME/.bashrc"
            updated=1
        fi
    fi

    # Update bash_profile (macOS)
    if [ -f "$HOME/.bash_profile" ]; then
        if ! grep -q "$install_dir" "$HOME/.bash_profile" 2>/dev/null; then
            echo "" >> "$HOME/.bash_profile"
            echo "# Added by bu installer" >> "$HOME/.bash_profile"
            echo "$path_line" >> "$HOME/.bash_profile"
            updated=1
        fi
    fi

    # Update zshrc
    if [ -f "$HOME/.zshrc" ]; then
        if ! grep -q "$install_dir" "$HOME/.zshrc" 2>/dev/null; then
            echo "" >> "$HOME/.zshrc"
            echo "# Added by bu installer" >> "$HOME/.zshrc"
            echo "$path_line" >> "$HOME/.zshrc"
            updated=1
        fi
    fi

    if [ $updated -eq 1 ]; then
        log_success "Added $install_dir to PATH in shell config"
        log_info "Restart your shell or run: source ~/.bashrc (or ~/.zshrc)"
    fi
}

# Cleanup function
cleanup() {
    if [ -n "$TMP_DIR" ] && [ -d "$TMP_DIR" ]; then
        rm -rf "$TMP_DIR"
    fi
}

# Main installation function
main() {
    VERSION=""
    UPDATE_PATH=1

    # Parse arguments
    while [ $# -gt 0 ]; do
        case "$1" in
            -h|--help)
                print_help
                exit 0
                ;;
            -d|--install-dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            -v|--version)
                VERSION="$2"
                shift 2
                ;;
            --no-path-update)
                UPDATE_PATH=0
                shift
                ;;
            --repo)
                REPO="$2"
                shift 2
                ;;
            *)
                log_error "Unknown option: $1"
                print_help
                exit 1
                ;;
        esac
    done

    # Setup cleanup trap
    trap cleanup EXIT INT TERM

    log_info "Installing bu CLI..."

    # Check dependencies
    check_dependencies

    # Detect platform and architecture
    PLATFORM=$(detect_platform)
    ARCH=$(detect_arch)
    TARGET="${ARCH}-${PLATFORM}"

    log_info "Detected target: $TARGET"

    # Get version
    if [ -z "$VERSION" ]; then
        log_info "Fetching latest version..."
        VERSION=$(get_latest_version)
    fi
    log_info "Installing version: $VERSION"

    # Create temporary directory
    TMP_DIR=$(mktemp -d)

    # Construct download URL
    BINARY_URL="https://github.com/$REPO/releases/download/$VERSION/bu-$TARGET"
    CHECKSUM_URL="https://github.com/$REPO/releases/download/$VERSION/bu-$TARGET.sha256"

    # Download binary
    download_file "$BINARY_URL" "$TMP_DIR/$BINARY_NAME"

    # Download and verify checksum (optional, don't fail if not available)
    if download_file "$CHECKSUM_URL" "$TMP_DIR/$BINARY_NAME.sha256" 2>/dev/null; then
        verify_checksum "$TMP_DIR/$BINARY_NAME" "$TMP_DIR/$BINARY_NAME.sha256"
    else
        log_warn "Checksum file not available, skipping verification"
    fi

    # Create install directory
    mkdir -p "$INSTALL_DIR"

    # Install binary
    cp "$TMP_DIR/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"

    log_success "Installed bu to $INSTALL_DIR/$BINARY_NAME"

    # Update PATH
    if [ $UPDATE_PATH -eq 1 ]; then
        update_shell_config "$INSTALL_DIR"
    fi

    # Verify installation
    if [ -x "$INSTALL_DIR/$BINARY_NAME" ]; then
        log_success "Installation complete!"
        echo ""
        echo "To get started, run:"
        echo "  $INSTALL_DIR/$BINARY_NAME --help"
    else
        log_error "Installation verification failed"
        exit 1
    fi
}

# Run main
main "$@"
