# Contributing to netwatch

Thank you for your interest in contributing to netwatch! This document provides guidelines and information for contributors.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Environment](#development-environment)
- [Code Style](#code-style)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)
- [Reporting Issues](#reporting-issues)

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally
3. Create a new branch for your feature or bugfix
4. Make your changes
5. Test your changes
6. Submit a pull request

## Development Environment

### Prerequisites

- Rust 1.70 or later
- Git
- A Unix-like system (Linux, macOS, or BSD)

### Setup

```bash
git clone https://github.com/vietcgi/netwatch
cd netwatch
cargo build
cargo test
```

### Running the Application

```bash
# Run with default options
cargo run

# Run with specific interface
cargo run eth0

# List available interfaces
cargo run -- --list

# Show help
cargo run -- --help
```

## Code Style

We use standard Rust formatting and linting tools:

```bash
# Format code
cargo fmt

# Check for common mistakes
cargo clippy

# Run all checks
cargo clippy --all-targets --all-features -- -D warnings
```

### Guidelines

- Follow Rust naming conventions
- Write documentation for public APIs
- Add unit tests for new functionality
- Keep functions focused and small
- Use descriptive variable names
- Add comments for complex logic

## Testing

We maintain comprehensive test coverage:

```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test integration

# Run with verbose output
cargo test --verbose

# Run benchmarks
cargo bench
```

### Test Types

- **Unit tests**: Test individual functions and modules
- **Integration tests**: Test CLI interface and end-to-end functionality
- **Benchmarks**: Performance tests for critical paths

### Writing Tests

- Add unit tests in the same file as the code being tested
- Add integration tests in the `tests/` directory
- Test both success and error cases
- Use descriptive test names
- Keep tests focused and independent

## Submitting Changes

### Pull Request Process

1. Ensure your code passes all tests and checks:
   ```bash
   cargo test
   cargo clippy --all-targets --all-features -- -D warnings
   cargo fmt -- --check
   ```

2. Update documentation if needed
3. Add changelog entry for significant changes
4. Submit pull request with clear description

### Commit Messages

Use clear, descriptive commit messages:

```
Add support for IPv6 interface monitoring

- Extend platform-specific implementations to handle IPv6
- Update tests to cover IPv6 scenarios
- Add configuration option for IPv6/IPv4 preference
```

### Branch Naming

Use descriptive branch names:
- `feature/ipv6-support`
- `bugfix/memory-leak-fix`
- `docs/improve-readme`

## Reporting Issues

### Bug Reports

When reporting bugs, please include:

- Operating system and version
- Rust version (`rustc --version`)
- Steps to reproduce the issue
- Expected vs actual behavior
- Error messages or logs
- Network interface information (`ip addr` on Linux, `ifconfig` on macOS)

### Feature Requests

When requesting features:

- Describe the use case
- Explain why the feature would be useful
- Provide examples of how it would be used
- Consider backward compatibility

### Security Issues

For security-related issues, please email the maintainers directly instead of opening a public issue.

## Development Guidelines

### Platform Support

When adding features:

- Ensure cross-platform compatibility (Linux and macOS)
- Use platform-specific implementations in `src/platform/`
- Test on both platforms when possible
- Document platform-specific limitations

### Performance

- Profile performance-critical code
- Use benchmarks to measure improvements
- Consider memory usage and allocations
- Optimize for typical network monitoring workloads

### Dependencies

- Minimize external dependencies
- Prefer well-maintained crates
- Check licenses for compatibility (MIT/Apache-2.0)
- Update dependencies regularly for security

## Code of Conduct

- Be respectful and inclusive
- Welcome newcomers and help them learn
- Focus on constructive feedback
- Collaborate openly and transparently

## Getting Help

- Open an issue for questions
- Check existing issues and documentation
- Join discussions in pull requests
- Ask questions in commit comments

## Recognition

Contributors will be recognized in:
- `AUTHORS` file
- Release notes
- GitHub contributors page

Thank you for contributing to netwatch!