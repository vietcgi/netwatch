# Changelog

## [0.1.2] - 2025-07-27

### Fixed
- Fixed security forensics crashes under heavy load and adverse conditions
- Added comprehensive panic protection to all forensics panel functions
- Implemented graceful fallback UI when forensics analysis fails
- Enhanced system stability during high traffic scenarios

### Changed
- Improved error handling in security analysis components
- Better resilience in network intelligence processing
- More robust connection monitoring under stress

## [0.1.1] - 2025-07-27

### Added
- Automatic glibc version detection in installation script
- Better platform compatibility for older Linux distributions
- High performance mode (`--high-perf`) for heavy traffic scenarios
- Adaptive polling intervals based on refresh rate

### Changed
- Default refresh interval increased from 500ms to 1000ms for better performance
- Installation script now automatically chooses musl static binaries for glibc < 2.35
- Improved user feedback during installation process
- Dashboard update intervals now scale with refresh rate for better CPU efficiency
- Event polling adapts to refresh rate to reduce CPU usage

### Fixed
- Compatibility issues with Ubuntu 20.04 and other older Linux distributions
- Installation script now works correctly on systems with older glibc versions
- ARM64 cross-compilation build issues in CI/CD pipeline
- Performance issues under heavy network traffic scenarios
- Installation script version parsing and asset detection

### Security
- Enhanced binary selection for better compatibility across Linux distributions

### Performance
- Reduced CPU usage under heavy traffic loads
- Better responsiveness with adaptive polling
- Configurable performance modes for different use cases
- Optimized security forensic system for high traffic scenarios
- Circular buffer implementation for security events to prevent memory issues
- Throttled security event processing under heavy load
- Auto-detection of high traffic conditions with automatic security optimization

All notable changes to netwatch will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial implementation of netwatch - a modern network traffic monitor
- Real-time network monitoring with beautiful terminal UI
- Cross-platform support (Linux and macOS)
- nload compatibility with all command-line options
- Interactive controls with keyboard shortcuts
- Multiple output formats (terminal UI, logging, CSV export)
- Active network diagnostics and health monitoring
- Connection and process tracking
- Performance bottleneck detection
- Comprehensive test suite
- GitHub Actions CI/CD pipeline
- Security auditing with cargo-deny and cargo-audit
- Documentation and examples

### Security
- Memory safety guaranteed by Rust
- No known security vulnerabilities
- Dependency security auditing in CI

## [0.1.0] - 2025-01-27

### Added
- **Initial Release** - First public release of netwatch
- **Core Monitoring** - Real-time network traffic monitoring for Linux and macOS
- **nload Compatibility** - Full compatibility with nload command-line options
- **SRE Dashboard** - Advanced network forensics and diagnostics interface
- **Active Diagnostics** - Real-time connectivity testing and health monitoring
- **Connection Tracking** - Monitor TCP/UDP connections with process information
- **Multiple Display Modes** - Dashboard, simple overview, and classic modes
- **Cross-platform Support** - Native Linux and macOS implementations
- **Modern Architecture** - Clean Rust implementation with memory safety
- **Interactive Controls** - Keyboard shortcuts for navigation and configuration
- **Export Capabilities** - Log to files in various formats
- **Comprehensive Testing** - Full test suite with CI/CD pipeline
- **Professional Documentation** - Complete guides and API documentation

### Security
- Memory safety guaranteed by Rust ownership system
- No known security vulnerabilities
- Comprehensive dependency security auditing
- Minimal privilege requirements

### Performance
- Low CPU overhead (0.2-2.1% typical usage)
- Minimal memory footprint (~2.4MB base)
- Efficient cross-platform implementations
- Battery-friendly operation