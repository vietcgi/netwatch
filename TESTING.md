# netwatch Testing Guide

## Overview

This document outlines the comprehensive testing strategy for netwatch, including manual testing procedures, automated CI/CD pipelines, and performance benchmarking.

## 🧪 Manual Testing on macOS (Current Results)

### ✅ **Successful Tests**
- **Compilation**: Builds cleanly on macOS 
- **CLI Interface**: All command-line options work correctly
- **Interface Detection**: Successfully lists 15 network interfaces
- **Help System**: Complete help documentation displays properly

### ⚠️ **Known Issues**
- **macOS Network Reading**: sysctl implementation needs refinement
- **Error**: "Device not configured (os error 6)" when reading interface statistics

### **Test Commands**
```bash
# Build the project
cargo build

# Test interface listing
./target/debug/netwatch --list

# Test help system
./target/debug/netwatch --help

# Test with specific interface (currently has issues)
./target/debug/netwatch en0
```

## 🤖 Automated CI/CD Pipeline

### **GitHub Actions Workflows**

#### 1. **Main CI Pipeline** (`.github/workflows/ci.yml`)
- **Platforms**: Ubuntu, macOS
- **Rust Versions**: stable, beta
- **Tests**:
  - Code formatting check
  - Clippy linting with strict warnings
  - Unit tests
  - Integration tests
  - CLI functionality tests
  - Platform-specific interface detection

#### 2. **Security Pipeline** (`.github/workflows/security.yml`)
- **Daily security audits** (2 AM UTC)
- **Dependency vulnerability scanning**
- **Supply chain security (SBOM generation)**
- **Static analysis with security-focused lints**
- **Fuzz testing** (scheduled runs)
- **Memory safety checks** with Miri

#### 3. **Performance Pipeline** (`.github/workflows/performance.yml`)
- **Benchmark suite** for statistics calculations
- **Memory usage profiling**
- **Load testing** with high-frequency monitoring
- **Performance regression detection** on PRs
- **CPU usage profiling**

#### 4. **Release Pipeline** (`.github/workflows/release.yml`)
- **Multi-platform binary builds**:
  - Linux x86_64 (glibc)
  - Linux x86_64 (musl, static)
  - macOS x86_64 (Intel)
  - macOS ARM64 (Apple Silicon)
- **Automated releases** to GitHub, crates.io
- **Homebrew formula updates**
- **Docker image builds** (multi-arch)

## 🔧 Cross-Platform Testing Strategy

### **Platform Matrix**
| Platform | Status | Testing Focus |
|----------|--------|---------------|
| **Linux** | ✅ Primary | `/proc/net/dev` parsing, high-volume interfaces |
| **macOS** | ⚠️ Partial | `sysctl` implementation, BSD interfaces |
| **FreeBSD** | 🔄 Planned | BSD socket statistics |
| **OpenBSD** | 🔄 Planned | BSD socket statistics |

### **Testing Environments**

#### **GitHub Actions Runners**
- **Ubuntu Latest**: Primary Linux testing
- **macOS Latest**: macOS compatibility and performance
- **Custom Runners**: Available for specialized testing

#### **Local Testing Environments**
```bash
# Docker testing for Linux variants
docker run --rm -it rust:1.75-slim bash
docker run --rm -it alpine:latest sh

# macOS testing (current environment)
cargo test --all-features
cargo bench
```

### **Virtual Network Interface Testing**
```bash
# Linux - Create dummy interfaces for testing
sudo ip link add test0 type dummy
sudo ip link add test1 type dummy
sudo ip link set test0 up
sudo ip link set test1 up

# macOS - Use existing interfaces
# Test with: lo0, en0, bridge0
```

## 📊 Performance Testing

### **Benchmark Suite**

#### **Statistics Benchmarks** (`benches/statistics.rs`)
- Single sample processing
- Batch processing (100 samples)
- Window trimming with historical data
- Counter overflow handling
- Graph data management

#### **Platform Benchmarks** (`benches/platform.rs`)
- Interface listing performance
- Statistics reading latency
- Multiple interface reading
- Platform availability checks

### **Performance Targets**
| Metric | Target | Measurement |
|--------|--------|-------------|
| **Single Sample Processing** | < 1μs | Statistics calculation |
| **Interface Listing** | < 10ms | Platform-specific calls |
| **Memory Usage** | < 10MB | Steady-state operation |
| **CPU Usage** | < 1% | Normal monitoring |

### **Load Testing Scenarios**
1. **High Frequency**: 50ms refresh rate for 30 seconds
2. **Multiple Interfaces**: Monitor 10+ interfaces simultaneously
3. **Long Duration**: 24-hour continuous monitoring
4. **Memory Stability**: No memory leaks over extended periods

## 🔒 Security Testing

### **Automated Security Checks**
- **Daily vulnerability scans** with `cargo audit`
- **Dependency analysis** with `cargo deny`
- **Static analysis** with Clippy security lints
- **Fuzz testing** for input parsing
- **SBOM generation** for supply chain transparency

### **Manual Security Review**
- **Privilege requirements**: Runs with user privileges only
- **Network access**: Read-only system statistics
- **File permissions**: Configuration files in user directory
- **Input validation**: Command-line arguments and config files

## 🏗️ Integration Testing

### **Test Scenarios**
1. **CLI Argument Validation**
   ```bash
   cargo test --test integration
   ```

2. **Configuration Loading**
   - `.netwatch` TOML format
   - `.nload` legacy format compatibility
   - Environment variable overrides

3. **Platform Integration**
   - Interface detection accuracy
   - Statistics reading consistency
   - Error handling for unavailable interfaces

4. **Output Format Testing**
   - Terminal UI rendering
   - Log file format compatibility
   - JSON/CSV export (future)

## 🚀 Deployment Testing

### **Package Testing**
```bash
# Local installation testing
cargo install --path .
netwatch --version

# Docker testing
docker build -t netwatch .
docker run --rm netwatch --help
```

### **Release Validation**
1. **Binary Distribution**: Download and test released binaries
2. **Package Managers**: Homebrew, Linux package repositories
3. **Container Images**: Docker Hub multi-arch images
4. **Documentation**: Verify all links and examples work

## 📈 Continuous Monitoring

### **Performance Tracking**
- **Benchmark results** archived in CI artifacts
- **Performance regression** detection on PRs
- **Memory usage trends** monitored over time

### **Quality Metrics**
- **Code coverage** reporting with Codecov
- **Security audit** results tracking
- **Dependency freshness** monitoring

## 🔧 Local Development Testing

### **Pre-commit Checks**
```bash
# Format check
cargo fmt --check

# Lint check
cargo clippy -- -D warnings

# Test suite
cargo test --all-features

# Security audit
cargo audit

# Benchmark verification
cargo bench --no-run
```

### **Manual Testing Procedures**
1. **Interface Detection**: Verify all network interfaces are listed
2. **Statistics Accuracy**: Compare with system tools (`ifconfig`, `ip`)
3. **Configuration**: Test both TOML and legacy formats
4. **Error Handling**: Test with invalid interfaces and configurations
5. **Performance**: Monitor resource usage during operation

## 📋 Test Results Summary

### **Current Status** (as of latest test)
- ✅ **Build System**: Compiles successfully on macOS and Linux
- ✅ **CLI Interface**: All command-line options functional  
- ✅ **Interface Detection**: Successfully lists network interfaces
- ⚠️ **Statistics Reading**: Linux working, macOS needs fixes
- ✅ **Configuration**: TOML and legacy format support
- ✅ **Testing Infrastructure**: Comprehensive CI/CD pipeline ready
- 🔄 **Performance**: Benchmark suite ready for execution

### **Next Steps**
1. **Fix macOS Implementation**: Resolve sysctl statistics reading
2. **Complete Terminal UI**: Implement graph rendering
3. **Add More Platforms**: FreeBSD, OpenBSD support
4. **Performance Optimization**: Based on benchmark results
5. **Security Hardening**: Based on audit findings

This testing strategy ensures netwatch maintains high quality, security, and performance across all supported platforms.