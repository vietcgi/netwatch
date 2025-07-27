# Netwatch Release Guide

This document outlines the release process for Netwatch, including automated builds, distribution, and installation methods.

## Release Process Overview

Netwatch uses an automated release process that triggers when a new version tag is pushed to the repository. The process includes:

1. **Cross-platform binary builds** for Linux and macOS
2. **Security verification** with checksums and signatures
3. **Multi-arch Docker images** for containerized deployments
4. **Package manager integration** (crates.io, Homebrew)
5. **Automated documentation** generation

## Supported Platforms

### Linux
- **x86_64 (Intel/AMD 64-bit)**
  - `netwatch-linux-x86_64.tar.gz` - Dynamic linking (glibc)
  - `netwatch-linux-x86_64-musl.tar.gz` - Static linking (musl)
- **ARM64 (Apple Silicon, ARM Cortex-A)**
  - `netwatch-linux-arm64.tar.gz` - Dynamic linking (glibc)
  - `netwatch-linux-arm64-musl.tar.gz` - Static linking (musl)

### macOS
- **x86_64 (Intel Macs)**
  - `netwatch-macos-x86_64.tar.gz`
- **ARM64 (Apple Silicon Macs)**
  - `netwatch-macos-arm64.tar.gz`

### Docker
- **Multi-architecture support**
  - `linux/amd64` (x86_64)
  - `linux/arm64` (ARM64)

## Installation Methods

### 1. Automated Installation Script (Recommended)

The easiest way to install netwatch:

```bash
# Install latest version
curl -sSL https://raw.githubusercontent.com/vietcgi/netwatch/main/install.sh | bash

# Or with wget
wget -qO- https://raw.githubusercontent.com/vietcgi/netwatch/main/install.sh | bash

# Install to custom directory
INSTALL_DIR=~/.local/bin curl -sSL https://raw.githubusercontent.com/vietcgi/netwatch/main/install.sh | bash

# Install specific version
curl -sSL https://raw.githubusercontent.com/vietcgi/netwatch/main/install.sh | bash -s -- --version v0.1.0
```

### 2. Manual Binary Download

1. Visit the [Releases page](https://github.com/vietcgi/netwatch/releases)
2. Download the appropriate binary for your platform
3. Extract and install:

```bash
# Example for Linux x86_64
wget https://github.com/vietcgi/netwatch/releases/latest/download/netwatch-linux-x86_64.tar.gz
tar -xzf netwatch-linux-x86_64.tar.gz
sudo mv netwatch /usr/local/bin/
chmod +x /usr/local/bin/netwatch
```

### 3. Package Managers

#### Rust/Cargo
```bash
cargo install netwatch
```

#### Homebrew (macOS)
```bash
brew install netwatch
```

#### Docker
```bash
# Run directly
docker run --rm ghcr.io/vietcgi/netwatch:latest --help

# Interactive monitoring (requires host network access)
docker run --rm -it --net=host ghcr.io/vietcgi/netwatch:latest eth0
```

### 4. Build from Source
```bash
git clone https://github.com/vietcgi/netwatch.git
cd netwatch
cargo build --release
sudo cp target/release/netwatch /usr/local/bin/
```

## Security Verification

All release binaries include checksums for integrity verification:

```bash
# Download binary and checksums
wget https://github.com/vietcgi/netwatch/releases/latest/download/netwatch-linux-x86_64.tar.gz
tar -xzf netwatch-linux-x86_64.tar.gz

# Verify checksums (included in the tarball)
sha256sum -c netwatch-linux-x86_64.sha256
sha512sum -c netwatch-linux-x86_64.sha512
```

## Release Workflow Details

### Triggering a Release

Releases are triggered by pushing a version tag:

```bash
# Create and push a new version tag
git tag v0.2.0
git push origin v0.2.0
```

### Automated Processes

The release workflow automatically:

1. **Creates GitHub Release**
   - Generates release notes
   - Attaches all platform binaries
   - Includes checksums and signatures

2. **Builds Cross-Platform Binaries**
   - Linux x86_64 (glibc + musl)
   - Linux ARM64 (glibc + musl) 
   - macOS x86_64 + ARM64
   - Strips symbols for smaller size
   - Generates SHA256/SHA512 checksums

3. **Publishes to Package Managers**
   - Publishes to crates.io
   - Updates Homebrew formula
   - Pushes Docker images to registry

4. **Security Scanning**
   - Runs security audits
   - Generates SBOM (Software Bill of Materials)
   - Performs vulnerability scanning

### Build Matrix

| Platform | Target | Static | Cross-compile | Runner |
|----------|--------|---------|---------------|---------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | No | No | ubuntu-latest |
| Linux x86_64 (static) | `x86_64-unknown-linux-musl` | Yes | Yes | ubuntu-latest |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | No | Yes | ubuntu-latest |
| Linux ARM64 (static) | `aarch64-unknown-linux-musl` | Yes | Yes | ubuntu-latest |
| macOS x86_64 | `x86_64-apple-darwin` | No | No | macos-latest |
| macOS ARM64 | `aarch64-apple-darwin` | No | Yes | macos-latest |

## Version Management

Netwatch follows [Semantic Versioning](https://semver.org/):

- **Major version** (X.0.0): Breaking changes
- **Minor version** (0.X.0): New features, backward compatible
- **Patch version** (0.0.X): Bug fixes, backward compatible

### Version Bumping Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md` with release notes
3. Commit changes: `git commit -m "Release v0.2.0"`
4. Create and push tag: `git tag v0.2.0 && git push origin v0.2.0`
5. Monitor the automated release workflow

## Docker Image Variants

### Tags Available
- `latest` - Latest stable release
- `v0.1.0` - Specific version tags
- `0.1` - Minor version tags
- `0` - Major version tags

### Multi-Architecture Support
All Docker images support both `linux/amd64` and `linux/arm64` architectures automatically.

### Usage Examples
```bash
# Basic usage
docker run --rm ghcr.io/vietcgi/netwatch:latest --help

# Monitor network interface (requires host network)
docker run --rm -it --net=host ghcr.io/vietcgi/netwatch:latest

# Run with custom configuration
docker run --rm -v /path/to/config:/config ghcr.io/vietcgi/netwatch:latest --config /config/netwatch.toml
```

## Package Manager Integration

### Crates.io
Automatically published on each release. Users can install with:
```bash
cargo install netwatch
```

### Homebrew
Formula is automatically updated in the `homebrew-tap` repository:
```bash
brew install vietcgi/tap/netwatch
```

## Quality Assurance

Each release goes through:

1. **Automated Testing**
   - Unit tests across all modules
   - Integration tests on multiple platforms
   - Security integration tests
   - Performance benchmarks

2. **Security Scanning**
   - Dependency vulnerability scanning
   - SAST (Static Application Security Testing)
   - Container security scanning
   - SBOM generation for supply chain security

3. **Cross-Platform Verification**
   - Builds tested on Linux (Ubuntu)
   - Builds tested on macOS (latest)
   - Docker images tested on multiple architectures

## Troubleshooting Releases

### Common Issues

1. **Build Failures**
   - Check GitHub Actions workflow logs
   - Verify all dependencies are compatible
   - Ensure Rust version requirements are met

2. **Missing Binaries**
   - Check if all target platforms built successfully
   - Verify cross-compilation tools are working
   - Check for permission issues in artifact upload

3. **Package Manager Issues**
   - Verify API tokens are valid and have correct permissions
   - Check rate limits on package registries
   - Ensure formula/package definitions are correct

### Manual Release Recovery

If automated release fails, manually create release:

```bash
# Build all targets locally
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-musl
cross build --release --target aarch64-unknown-linux-gnu
cross build --release --target aarch64-unknown-linux-musl

# Create archives with checksums
for target in x86_64-unknown-linux-gnu x86_64-unknown-linux-musl aarch64-unknown-linux-gnu aarch64-unknown-linux-musl; do
    cd target/$target/release
    sha256sum netwatch > netwatch-$target.sha256
    sha512sum netwatch > netwatch-$target.sha512
    tar czf netwatch-$target.tar.gz netwatch netwatch-$target.sha256 netwatch-$target.sha512
    cd ../../..
done

# Upload manually to GitHub Release
gh release create v0.1.0 target/*/release/netwatch-*.tar.gz
```

## Release Checklist

Before releasing:

- [ ] All tests pass on CI
- [ ] Documentation is up to date
- [ ] CHANGELOG.md is updated
- [ ] Version bumped in Cargo.toml
- [ ] Security audit passes
- [ ] Performance benchmarks acceptable
- [ ] Manual testing completed

After releasing:

- [ ] Verify all binaries downloaded and work
- [ ] Test installation script
- [ ] Verify package manager updates
- [ ] Update documentation sites
- [ ] Announce release in relevant channels