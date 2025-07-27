# Secure Development Guidelines for Netwatch

## Pre-Development Security Checklist

### Environment Setup
- [ ] Latest Rust toolchain with security updates
- [ ] Security linting tools installed (`cargo clippy`, `cargo audit`)
- [ ] Pre-commit hooks configured for security scanning
- [ ] Development environment isolated from production

### Code Security Standards

#### Input Validation
```rust
// ✅ GOOD: Validate network interface names
fn validate_interface_name(name: &str) -> Result<(), NetwatchError> {
    if name.is_empty() || name.len() > 16 {
        return Err(NetwatchError::Parse("Invalid interface name".to_string()));
    }
    
    // Only allow alphanumeric and common interface characters
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(NetwatchError::Parse("Invalid characters in interface name".to_string()));
    }
    
    Ok(())
}

// ❌ BAD: Direct use without validation
fn read_interface_stats(name: &str) -> Result<Stats> {
    let path = format!("/proc/net/dev/{}", name); // Potential path traversal
    std::fs::read_to_string(path) // Unsafe
}
```

#### Error Handling
```rust
// ✅ GOOD: Structured error types that don't leak sensitive info
#[derive(Debug, thiserror::Error)]
pub enum NetwatchError {
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    
    #[error("Permission denied")]
    PermissionDenied, // No sensitive details
    
    #[error("IO error")]
    Io(#[from] std::io::Error), // Filtered error
}

// ❌ BAD: Exposing internal details
#[error("Failed to read /proc/net/dev: {internal_error}")]
InternalError { internal_error: String },
```

#### Configuration Security
```rust
// ✅ GOOD: Safe configuration parsing
fn load_config() -> Result<Config> {
    let config_path = dirs::home_dir()
        .ok_or(NetwatchError::Config("No home directory".to_string()))?
        .join(".netwatch");
    
    if !config_path.exists() {
        return Ok(Config::default());
    }
    
    // Validate file permissions before reading
    let metadata = config_path.metadata()
        .map_err(|_| NetwatchError::Config("Cannot read config metadata".to_string()))?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        if mode & 0o077 != 0 {
            return Err(NetwatchError::Config("Config file too permissive".to_string()));
        }
    }
    
    let content = std::fs::read_to_string(&config_path)
        .map_err(|_| NetwatchError::Config("Cannot read config file".to_string()))?;
    
    toml::from_str(&content)
        .map_err(|e| NetwatchError::Config(format!("Invalid config: {}", e)))
}
```

## Security Testing Guidelines

### Unit Testing Security
```rust
#[cfg(test)]
mod security_tests {
    use super::*;
    
    #[test]
    fn test_interface_name_validation() {
        // Test valid names
        assert!(validate_interface_name("eth0").is_ok());
        assert!(validate_interface_name("wlan0").is_ok());
        
        // Test invalid names
        assert!(validate_interface_name("").is_err());
        assert!(validate_interface_name("../../../etc/passwd").is_err());
        assert!(validate_interface_name("interface_with_very_long_name_that_exceeds_limit").is_err());
        assert!(validate_interface_name("interface with spaces").is_err());
        assert!(validate_interface_name("interface\x00null").is_err());
    }
    
    #[test]
    fn test_config_file_permissions() {
        // Test that restrictive permissions are enforced
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".netwatch");
        
        std::fs::write(&config_path, "devices = \"eth0\"").unwrap();
        
        // Set permissive permissions (should fail)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = config_path.metadata().unwrap().permissions();
            perms.set_mode(0o644); // World readable
            std::fs::set_permissions(&config_path, perms).unwrap();
            
            assert!(load_config_from_path(&config_path).is_err());
        }
    }
}
```

### Fuzz Testing Setup
```rust
// fuzz/fuzz_targets/stats_parsing.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use netwatch::platform::LinuxReader;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let reader = LinuxReader::new();
        // This should never panic, only return errors
        let _ = reader.parse_proc_net_dev(input, "eth0");
    }
});
```

### Integration Testing
```rust
#[cfg(test)]
mod integration_tests {
    use assert_cmd::Command;
    use predicates::prelude::*;
    
    #[test]
    fn test_no_privilege_escalation() {
        let mut cmd = Command::cargo_bin("netwatch").unwrap();
        cmd.arg("--list")
            .assert()
            .success(); // Should work without root
    }
    
    #[test]
    fn test_input_sanitization() {
        let mut cmd = Command::cargo_bin("netwatch").unwrap();
        cmd.arg("--devices").arg("../../../etc/passwd")
            .assert()
            .failure()
            .stderr(predicate::str::contains("Invalid interface name"));
    }
}
```

## Security Review Process

### Pre-Commit Checklist
- [ ] No hardcoded secrets or credentials
- [ ] Input validation for all user inputs
- [ ] Proper error handling without information leakage
- [ ] No unsafe code blocks
- [ ] Dependencies updated and scanned
- [ ] Tests cover security edge cases

### Code Review Security Focus
1. **Input Validation**: Every user input validated
2. **Error Handling**: No sensitive information in error messages
3. **File Operations**: Proper path validation and permissions
4. **Network Operations**: Validate all network-related inputs
5. **Configuration**: Secure configuration loading and validation

### Security Testing Commands
```bash
# Run all security checks
cargo clippy -- -D warnings
cargo audit
cargo deny check
cargo test

# Fuzz testing (run periodically)
cargo fuzz run stats_parsing -- -max_total_time=300

# Generate security report
cargo cyclonedx --format json > security/sbom.json
```

## Dependency Security

### Approved Dependencies
- **Core**: `serde`, `toml`, `anyhow`, `thiserror` - Well-maintained serialization and error handling
- **UI**: `ratatui`, `crossterm` - Actively maintained terminal UI libraries
- **CLI**: `clap` - Secure command-line argument parsing
- **System**: `libc`, `dirs` - Standard system interaction libraries

### Dependency Review Process
1. **New Dependencies**: Security review required
2. **Updates**: Automated security scanning
3. **Removal**: Impact assessment for security implications

### Banned Dependencies
- Any crate marked as unmaintained for >6 months
- Crates with known security vulnerabilities
- Crates with excessive permissions or capabilities

## Secure Deployment

### Build Security
```bash
# Secure build process
cargo build --release --locked
cargo audit
cargo deny check

# Verify build reproducibility
export SOURCE_DATE_EPOCH=$(git log -1 --format=%ct)
cargo build --release
```

### Distribution Security
- **Checksums**: Provide SHA256 checksums for all releases
- **Signatures**: Sign releases with GPG keys
- **Supply Chain**: Verify build environment integrity

### Runtime Security Recommendations
1. **User Privileges**: Run as non-privileged user
2. **File Permissions**: Restrict config file permissions (600)
3. **Network Isolation**: Use network namespaces if available
4. **Resource Limits**: Apply ulimits for process resources

## Incident Response for Developers

### Security Bug Discovery
1. **Immediate**: Stop development of affected feature
2. **Assessment**: Evaluate severity and impact
3. **Containment**: Develop fix in private branch
4. **Testing**: Comprehensive security testing of fix
5. **Disclosure**: Coordinate with security team

### Communication Protocol
- **Internal**: Use secure channels for sensitive discussions
- **External**: Follow responsible disclosure guidelines
- **Documentation**: Maintain security incident log

---

**Security Contact**: security@netwatch.project  
**Emergency Contact**: [Security Lead Phone]  
**Last Updated**: 2025-07-27