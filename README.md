# netwatch

[![CI](https://github.com/vietcgi/netwatch/workflows/CI/badge.svg)](https://github.com/vietcgi/netwatch/actions)
[![Security](https://github.com/vietcgi/netwatch/workflows/Security/badge.svg)](https://github.com/vietcgi/netwatch/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A modern network traffic monitor for Unix systems, inspired by nload but written in Rust with enhanced features and beautiful terminal interfaces.

## ✨ Features

### Core Monitoring
- **Real-time network monitoring** - Live bandwidth and packet statistics
- **Multi-interface support** - Monitor multiple network interfaces simultaneously
- **Cross-platform** - Native support for Linux and macOS
- **nload compatibility** - Drop-in replacement with all nload command-line options

### Advanced Analytics
- **SRE Dashboard** - Advanced network forensics and diagnostics
- **Active Diagnostics** - Real-time connectivity testing and health monitoring
- **Connection Tracking** - Monitor TCP/UDP connections with process information
- **Performance Analysis** - Bottleneck detection and network quality metrics
- **System Integration** - CPU, memory, and disk usage correlation

### Modern Interface
- **Beautiful Terminal UI** - Rich, colorful terminal interface with graphs
- **Multiple Display Modes** - Dashboard, simple overview, or classic nload-style
- **Interactive Controls** - Keyboard shortcuts for navigation and configuration
- **Export Capabilities** - Log to files in various formats

## Quick Start

### Installation

```bash
# From source (recommended)
git clone https://github.com/vietcgi/netwatch
cd netwatch
cargo install --path .

# From crates.io (when published)
cargo install netwatch
```

### Basic Usage

```bash
# Auto-detect and monitor default interface
netwatch

# List available interfaces
netwatch --list

# Monitor specific interface
netwatch en0

# SRE forensics dashboard mode
netwatch --sre-terminal

# Simple overview mode
netwatch --show-overview

# Monitor multiple interfaces
netwatch -m
```

## 📊 Display Modes

### 1. SRE Dashboard (Default)
Advanced network forensics interface with:
- Real-time connection analysis
- Active diagnostics and health checks
- Performance bottleneck detection
- System resource correlation
- Security monitoring alerts

### 2. Simple Overview
Clean, minimal interface showing:
- Interface statistics
- Bandwidth utilization
- Packet counts and rates
- Error summaries

### 3. Multi-Interface Mode
Monitor multiple interfaces with:
- Side-by-side comparisons
- Aggregate statistics
- Per-interface details

## ⚙️ Command Line Options

### Core Options (nload compatible)
```bash
-l, --list                    List available network interfaces
-a, --average <seconds>       Average window length [default: 300]
-i, --incoming <kBit/s>       Max incoming bandwidth scale (0 = auto)
-o, --outgoing <kBit/s>       Max outgoing bandwidth scale (0 = auto)
-t, --interval <ms>           Refresh interval in milliseconds [default: 500]
-u, --unit <unit>             Traffic unit format [default: k]
-U, --data-unit <unit>        Data unit for totals [default: M]
-m, --multiple                Show multiple devices
-f, --file <path>             Log traffic data to file
```

### Display Modes
```bash
--sre-terminal               SRE forensics dashboard mode
--show-overview              Simple overview mode
--debug-dashboard            Debug mode with detailed metrics
--test                       Test mode - single output and exit
--force-terminal             Force terminal mode (no TUI)
```

### Unit Formats
- `h` - Human-readable bits (auto-scaling)
- `H` - Human-readable bytes (auto-scaling)
- `k`/`K` - Kilobits/Kilobytes
- `m`/`M` - Megabits/Megabytes
- `g`/`G` - Gigabits/Gigabytes
- `b`/`B` - Raw bits/bytes

## 🎮 Interactive Controls

### Navigation
- **Arrow keys** - Navigate between interfaces/sections
- **Tab** - Switch between dashboard panels
- **Enter** - Select/drill down into details

### Display Controls
- **Space** - Pause/resume monitoring
- **r** - Reset statistics
- **g** - Toggle graph display
- **+/-** - Zoom graph scale
- **u** - Cycle through unit formats

### System Controls
- **F2** - Show options/settings
- **F5** - Save current configuration
- **F6** - Reload configuration
- **q** or **Ctrl+C** - Quit

## 📁 Configuration

### Configuration Files
Configuration is stored in TOML format:
- `~/.netwatch/config.toml` - Primary configuration
- `~/.nload` - nload compatibility mode

### Example Configuration
```toml
# ~/.netwatch configuration file
AverageWindow = 300
BarMaxIn = 0
BarMaxOut = 0
DataFormat = "M"
Devices = "all"
MultipleDevices = false
RefreshInterval = 500
TrafficFormat = "k"

# Active Diagnostics Configuration
# These targets will be tested for connectivity and performance
DiagnosticTargets = [
    "1.1.1.1",           # Cloudflare DNS (fast, privacy-focused)
    "8.8.8.8",           # Google DNS (widely accessible)
    "9.9.9.9"            # Quad9 DNS (security-focused)
]

# DNS domains to test for resolution performance
DNSDomains = [
    "cloudflare.com",    # Reliable test domain
    "google.com",        # Reliable test domain
    "github.com"         # Development-relevant domain
]
```

**Note**: See `example.netwatch` in the repository for a complete configuration template.

## 🔧 Building from Source

### Requirements
- **Rust 1.70+** - Latest stable Rust toolchain
- **Unix-like system** - Linux, macOS, or BSD
- **Development tools** - git, cargo

### Build Process
```bash
# Clone repository
git clone https://github.com/vietcgi/netwatch
cd netwatch

# Build release version
cargo build --release

# Run tests
cargo test

# Install locally
cargo install --path .
```

### Development
```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run specific interface
cargo run -- en0

# Run tests with output
cargo test -- --nocapture

# Format code
cargo fmt

# Lint code
cargo clippy
```

## 🧪 Testing

### Test Interface
```bash
# Test mode - single output and exit
netwatch --test

# Debug dashboard without TUI
netwatch --debug-dashboard

# List interfaces (good for CI)
netwatch --list
```

## 📈 Performance

- **Memory efficient** - Rust's zero-cost abstractions
- **Low CPU overhead** - Optimized for continuous monitoring
- **Scalable** - Handles hundreds of network interfaces
- **Battery friendly** - Configurable refresh intervals

## 🔒 Security

- **Memory safe** - Rust prevents buffer overflows and memory leaks
- **Privilege separation** - Runs with minimal required permissions
- **No network transmission** - Only reads local system statistics
- **Input validation** - All inputs are validated and sanitized

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Quick Start for Contributors
```bash
git clone https://github.com/vietcgi/netwatch
cd netwatch
cargo test
cargo clippy
cargo fmt
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by [nload](https://github.com/rolandriegel/nload) by Roland Riegel
- Built with [ratatui](https://github.com/ratatui-org/ratatui) for terminal UI
- Uses [clap](https://github.com/clap-rs/clap) for command-line parsing

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/vietcgi/netwatch/issues)
- **Security**: See [SECURITY.md](SECURITY.md) for reporting security issues
- **Discussions**: [GitHub Discussions](https://github.com/vietcgi/netwatch/discussions)

---

**netwatch** - Modern network monitoring for the terminal era
