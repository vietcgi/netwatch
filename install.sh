#!/bin/bash

# Netwatch Installation Script
# Downloads and installs the latest release of netwatch for your platform

set -euo pipefail

# Configuration
REPO="vietcgi/netwatch"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARY_NAME="netwatch"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

fatal() {
    error "$1"
    exit 1
}

# Detect platform and architecture
detect_platform() {
    local os arch
    
    case "$(uname -s)" in
        Linux*)
            os="linux"
            ;;
        Darwin*)
            os="macos"
            ;;
        *)
            fatal "Unsupported operating system: $(uname -s). Netwatch only supports Linux and macOS."
            ;;
    esac
    
    case "$(uname -m)" in
        x86_64|amd64)
            arch="x86_64"
            ;;
        aarch64|arm64)
            arch="arm64"
            ;;
        *)
            fatal "Unsupported architecture: $(uname -m). Netwatch only supports x86_64 and ARM64."
            ;;
    esac
    
    # Determine static vs dynamic linking preference for Linux
    if [[ "$os" == "linux" ]]; then
        # Check glibc version for compatibility
        local glibc_version=""
        if command -v ldd >/dev/null 2>&1; then
            # Extract glibc version
            glibc_version=$(ldd --version 2>/dev/null | head -n1 | grep -oE '[0-9]+\.[0-9]+' | head -n1 || echo "")
        fi
        
        # Use musl static binary for older glibc versions or when glibc detection fails
        if [[ -z "$glibc_version" ]] || [[ "$glibc_version" < "2.35" ]]; then
            info "Detected older glibc version ($glibc_version) or minimal system - using static musl binary for better compatibility"
            echo "${os}-${arch}-musl"
        else
            info "Detected modern glibc version ($glibc_version) - using dynamic binary"
            echo "${os}-${arch}"
        fi
    else
        echo "${os}-${arch}"
    fi
}

# Get the latest release version
get_latest_version() {
    info "Fetching latest release information..."
    
    local api_url="https://api.github.com/repos/${REPO}/releases/latest"
    local version
    
    if command -v curl >/dev/null 2>&1; then
        version=$(curl -s "$api_url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\\1/')
    elif command -v wget >/dev/null 2>&1; then
        version=$(wget -qO- "$api_url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\\1/')
    else
        fatal "Neither curl nor wget found. Please install one of them to continue."
    fi
    
    if [[ -z "$version" ]]; then
        fatal "Failed to fetch latest version information"
    fi
    
    echo "$version"
}

# Download and verify the binary
download_and_verify() {
    local version="$1"
    local platform="$2"
    local asset_name="netwatch-${platform}"
    local download_url="https://github.com/${REPO}/releases/download/${version}/${asset_name}.tar.gz"
    local checksum_url="https://github.com/${REPO}/releases/download/${version}/${asset_name}.sha256"
    
    info "Downloading netwatch ${version} for ${platform}..."
    info "Download URL: ${download_url}"
    
    # Create temporary directory
    local temp_dir
    temp_dir=$(mktemp -d)
    trap "rm -rf '$temp_dir'" EXIT
    
    cd "$temp_dir"
    
    # Download the archive and checksum
    if command -v curl >/dev/null 2>&1; then
        curl -sL "$download_url" -o "${asset_name}.tar.gz"
        curl -sL "$checksum_url" -o "${asset_name}.sha256" 2>/dev/null || warning "Checksum file not available for verification"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$download_url" -O "${asset_name}.tar.gz"
        wget -q "$checksum_url" -O "${asset_name}.sha256" 2>/dev/null || warning "Checksum file not available for verification"
    fi
    
    # Verify checksum if available
    if [[ -f "${asset_name}.sha256" ]]; then
        info "Verifying download integrity..."
        if command -v sha256sum >/dev/null 2>&1; then
            if ! sha256sum -c "${asset_name}.sha256" >/dev/null 2>&1; then
                fatal "Checksum verification failed. The download may be corrupted or tampered with."
            fi
            success "Checksum verification passed"
        else
            warning "sha256sum not available, skipping checksum verification"
        fi
    fi
    
    # Extract the archive
    info "Extracting archive..."
    tar -xzf "${asset_name}.tar.gz"
    
    # Verify the binary exists
    if [[ ! -f "$BINARY_NAME" ]]; then
        fatal "Binary not found in archive"
    fi
    
    echo "$temp_dir/$BINARY_NAME"
}

# Install the binary
install_binary() {
    local binary_path="$1"
    local install_path="${INSTALL_DIR}/${BINARY_NAME}"
    
    info "Installing netwatch to ${install_path}..."
    
    # Check if install directory exists and is writable
    if [[ ! -d "$INSTALL_DIR" ]]; then
        fatal "Install directory $INSTALL_DIR does not exist"
    fi
    
    if [[ ! -w "$INSTALL_DIR" ]]; then
        if [[ "$EUID" -eq 0 ]]; then
            fatal "Cannot write to $INSTALL_DIR even as root"
        else
            warning "No write permission to $INSTALL_DIR, trying with sudo..."
            sudo cp "$binary_path" "$install_path"
            sudo chmod 755 "$install_path"
        fi
    else
        cp "$binary_path" "$install_path"
        chmod 755 "$install_path"
    fi
    
    # Verify installation
    if [[ -x "$install_path" ]]; then
        success "Netwatch installed successfully to $install_path"
        
        # Test the binary
        if "$install_path" --version >/dev/null 2>&1; then
            success "Installation verified - netwatch is working correctly"
        else
            warning "Installation completed but binary test failed"
        fi
    else
        fatal "Installation failed - binary not found or not executable"
    fi
}

# Check dependencies
check_dependencies() {
    local missing_deps=()
    
    # Check for required tools
    if ! command -v tar >/dev/null 2>&1; then
        missing_deps+=("tar")
    fi
    
    if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
        missing_deps+=("curl or wget")
    fi
    
    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        fatal "Missing required dependencies: ${missing_deps[*]}"
    fi
}

# Print usage information
print_usage() {
    cat << EOF
Netwatch Installation Script

USAGE:
    $0 [OPTIONS]

OPTIONS:
    -d, --install-dir DIR    Installation directory (default: /usr/local/bin)
    -h, --help              Show this help message
    -v, --version VERSION   Install specific version (default: latest)

EXAMPLES:
    # Install latest version to default location
    $0
    
    # Install to custom directory
    INSTALL_DIR=~/.local/bin $0
    
    # Install specific version
    $0 --version v0.1.0

ENVIRONMENT VARIABLES:
    INSTALL_DIR             Installation directory (default: /usr/local/bin)

EOF
}

# Main installation function
main() {
    local version=""
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -d|--install-dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            -v|--version)
                version="$2"
                shift 2
                ;;
            -h|--help)
                print_usage
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                print_usage
                exit 1
                ;;
        esac
    done
    
    info "Starting netwatch installation..."
    info "Install directory: $INSTALL_DIR"
    
    # Check dependencies
    check_dependencies
    
    # Detect platform
    local platform
    platform=$(detect_platform)
    info "Detected platform: $platform"
    
    # Get version
    if [[ -z "$version" ]]; then
        version=$(get_latest_version)
    fi
    info "Installing version: $version"
    
    # Download and verify
    local binary_path
    binary_path=$(download_and_verify "$version" "$platform")
    
    # Install
    install_binary "$binary_path"
    
    # Final instructions
    echo
    success "Netwatch installation completed!"
    echo
    info "To get started:"
    echo "  netwatch --help          # Show help"
    echo "  netwatch --list          # List network interfaces"
    echo "  netwatch eth0            # Monitor specific interface"
    echo "  netwatch                 # Monitor default interface"
    echo
    
    # Check if install directory is in PATH
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        warning "⚠️  $INSTALL_DIR is not in your PATH"
        echo "   Add it to your PATH by adding this line to your shell profile:"
        echo "   export PATH=\"$INSTALL_DIR:\$PATH\""
    fi
}

# Run main function
main "$@"