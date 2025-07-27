# Installation Guide

This guide covers different methods to install netwatch on your system.

## Quick Install

### From Source (Recommended)
```bash
git clone https://github.com/vietcgi/netwatch
cd netwatch
cargo install --path .
```

### From crates.io (Future)
```bash
cargo install netwatch
```

## System Requirements

### Supported Platforms
- **Linux** - Any modern distribution
- **macOS** - 10.12 Sierra or later
- **Other Unix** - Should work on most Unix-like systems

### Dependencies
- **Rust 1.70+** - Latest stable Rust toolchain
- **Development tools** - git, cargo (included with Rust)

## Installation Methods

### 1. Cargo Install (Recommended)

Install directly from the repository:
```bash
# Install latest from repository
cargo install --git https://github.com/vietcgi/netwatch

# Install specific version (when available on crates.io)
cargo install netwatch --version 0.1.0
```

### 2. Manual Build

For development or customization:
```bash
# Clone and build
git clone https://github.com/vietcgi/netwatch
cd netwatch

# Build release version
cargo build --release

# Copy binary to PATH
sudo cp target/release/netwatch /usr/local/bin/
```

### 3. Package Managers (Future)

Coming soon:
```bash
# Homebrew (macOS/Linux)
brew install netwatch

# Arch Linux AUR
yay -S netwatch

# Debian/Ubuntu
apt install netwatch
```

## Post-Installation

### Verify Installation
```bash
# Check version
netwatch --version

# List available interfaces
netwatch --list

# Show help
netwatch --help
```

### Configuration
Netwatch will create configuration files on first run:
- `~/.netwatch/config.toml` - Main configuration
- `~/.nload` - nload compatibility (if exists)

### Permissions

Netwatch requires read access to network statistics:
- **Linux**: `/proc/net/dev` (usually available to all users)
- **macOS**: System network APIs (may require elevated privileges for some features)

For most monitoring features, no special permissions are needed.

## Troubleshooting

### Common Issues

#### "Permission denied" on macOS
Some network features may require elevated privileges:
```bash
sudo netwatch
```

#### "Interface not found"
List available interfaces first:
```bash
netwatch --list
```

#### Build failures
Ensure you have the latest Rust toolchain:
```bash
rustup update stable
```

### Dependencies Issues

If you encounter build errors, install platform dependencies:

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install build-essential pkg-config
```

**CentOS/RHEL:**
```bash
sudo yum groupinstall "Development Tools"
sudo yum install pkgconfig
```

**macOS:**
```bash
# Install Xcode command line tools
xcode-select --install
```

## Uninstallation

### Remove Binary
```bash
# If installed via cargo
cargo uninstall netwatch

# If manually installed
sudo rm /usr/local/bin/netwatch
```

### Remove Configuration
```bash
rm -rf ~/.netwatch
rm -f ~/.nload  # if exists
```

## Development Installation

For contributors and developers:

```bash
# Clone repository
git clone https://github.com/vietcgi/netwatch
cd netwatch

# Install development dependencies
cargo build

# Run tests
cargo test

# Install in development mode
cargo install --path . --debug
```

## Docker Usage

Run netwatch in a container:
```bash
# Build Docker image
docker build -t netwatch .

# Run with host networking
docker run --rm --net=host netwatch --list
```

Note: Container must use host networking to access network interfaces.

## Support

If you encounter installation issues:
- Check [GitHub Issues](https://github.com/vietcgi/netwatch/issues)
- Review [Troubleshooting Guide](TROUBLESHOOTING.md)
- Create a new issue with your system details

Include the following information in bug reports:
- Operating system and version
- Rust version (`rustc --version`)
- Error messages
- Network interface information