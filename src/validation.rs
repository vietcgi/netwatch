//! Input validation and sanitization for netwatch
//!
//! This module provides secure validation functions for all user inputs
//! to prevent injection attacks, path traversal, and other security issues.

use crate::error::{NetwatchError, Result};
use crate::security::{record_security_event, SecurityEvent};
use std::path::Path;

/// Maximum allowed length for network interface names
const MAX_INTERFACE_NAME_LEN: usize = 16;

/// Maximum allowed length for file paths
const MAX_PATH_LEN: usize = 4096;

/// Maximum allowed refresh interval in milliseconds
const MAX_REFRESH_INTERVAL: u64 = 60_000; // 1 minute

/// Minimum allowed refresh interval in milliseconds
const MIN_REFRESH_INTERVAL: u64 = 100; // 0.1 seconds

/// Validates network interface names to prevent path traversal and injection
///
/// # Security Considerations
/// - Prevents path traversal attacks (../../../etc/passwd)
/// - Blocks null bytes and control characters
/// - Limits length to prevent buffer overflow attacks
/// - Only allows safe characters commonly used in interface names
///
/// # Examples
/// ```
/// use netwatch::validation::validate_interface_name;
///
/// assert!(validate_interface_name("eth0").is_ok());
/// assert!(validate_interface_name("wlan0").is_ok());
/// assert!(validate_interface_name("../etc/passwd").is_err());
/// ```
pub fn validate_interface_name(name: &str) -> Result<()> {
    // Check for empty or overly long names
    if name.is_empty() {
        record_security_event(SecurityEvent::InvalidInput {
            input_type: "interface_name".to_string(),
            attempted_value: name.to_string(),
            source: "validation".to_string(),
        });
        return Err(NetwatchError::Parse(
            "Interface name cannot be empty".to_string(),
        ));
    }

    if name.len() > MAX_INTERFACE_NAME_LEN {
        record_security_event(SecurityEvent::InvalidInput {
            input_type: "interface_name".to_string(),
            attempted_value: name.to_string(),
            source: "validation".to_string(),
        });
        return Err(NetwatchError::Parse(format!(
            "Interface name too long (max {MAX_INTERFACE_NAME_LEN} characters)"
        )));
    }

    // Check for path traversal attempts
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        record_security_event(SecurityEvent::InvalidInput {
            input_type: "interface_name".to_string(),
            attempted_value: name.to_string(),
            source: "validation".to_string(),
        });
        return Err(NetwatchError::Parse(
            "Invalid characters in interface name".to_string(),
        ));
    }

    // Check for null bytes and control characters
    if name.contains('\0') || name.chars().any(|c| c.is_control()) {
        record_security_event(SecurityEvent::InvalidInput {
            input_type: "interface_name".to_string(),
            attempted_value: name.to_string(),
            source: "validation".to_string(),
        });
        return Err(NetwatchError::Parse(
            "Control characters not allowed in interface name".to_string(),
        ));
    }

    // Only allow alphanumeric characters, hyphens, underscores, and dots
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        record_security_event(SecurityEvent::InvalidInput {
            input_type: "interface_name".to_string(),
            attempted_value: name.to_string(),
            source: "validation".to_string(),
        });
        return Err(NetwatchError::Parse(
            "Invalid characters in interface name".to_string(),
        ));
    }

    // Additional security: block common attack patterns
    let name_lower = name.to_lowercase();
    if name_lower.contains("proc") || name_lower.contains("sys") || name_lower.contains("dev") {
        record_security_event(SecurityEvent::InvalidInput {
            input_type: "interface_name".to_string(),
            attempted_value: name.to_string(),
            source: "validation".to_string(),
        });
        return Err(NetwatchError::Parse(
            "Suspicious interface name pattern".to_string(),
        ));
    }

    Ok(())
}

/// Validates file paths for logging and configuration
///
/// # Security Considerations
/// - Prevents path traversal attacks
/// - Blocks access to sensitive system directories
/// - Validates file extensions for expected types
/// - Checks path length to prevent buffer overflow
///
/// # Examples
/// ```
/// use netwatch::validation::validate_file_path;
///
/// assert!(validate_file_path("/tmp/netwatch.log", Some("log")).is_ok());
/// assert!(validate_file_path("../../../etc/passwd", None).is_err());
/// ```
pub fn validate_file_path(path: &str, expected_extension: Option<&str>) -> Result<()> {
    if path.is_empty() {
        return Err(NetwatchError::Config(
            "File path cannot be empty".to_string(),
        ));
    }

    if path.len() > MAX_PATH_LEN {
        return Err(NetwatchError::Config(format!(
            "File path too long (max {MAX_PATH_LEN} characters)"
        )));
    }

    // Check for null bytes and control characters
    if path.contains('\0') || path.chars().any(|c| c.is_control()) {
        return Err(NetwatchError::Config(
            "Control characters not allowed in file path".to_string(),
        ));
    }

    let path_obj = Path::new(path);

    // Check for explicit path traversal patterns
    if path.contains("..") {
        record_security_event(SecurityEvent::InvalidInput {
            input_type: "file_path".to_string(),
            attempted_value: path.to_string(),
            source: "validation".to_string(),
        });
        return Err(NetwatchError::Config("Path traversal detected".to_string()));
    }

    // Block access to sensitive system directories
    let sensitive_dirs = [
        "/etc",
        "/boot",
        "/proc",
        "/sys",
        "/dev",
        "/root",
        "/usr/bin",
        "/usr/sbin",
        "/bin",
        "/sbin",
    ];

    for sensitive_dir in &sensitive_dirs {
        if path.starts_with(sensitive_dir) {
            return Err(NetwatchError::Config(
                "Access to sensitive directory denied".to_string(),
            ));
        }
    }

    // Validate file extension if specified
    if let Some(expected_ext) = expected_extension {
        if let Some(extension) = path_obj.extension() {
            if extension.to_string_lossy().to_lowercase() != expected_ext.to_lowercase() {
                return Err(NetwatchError::Config(format!(
                    "Invalid file extension, expected: {expected_ext}"
                )));
            }
        } else {
            return Err(NetwatchError::Config(format!(
                "Missing file extension, expected: {expected_ext}"
            )));
        }
    }

    Ok(())
}

/// Validates refresh interval values
///
/// # Security Considerations
/// - Prevents DoS attacks through excessive refresh rates
/// - Ensures reasonable bounds for system resource usage
/// - Prevents integer overflow in timing calculations
pub fn validate_refresh_interval(interval_ms: u64) -> Result<()> {
    if interval_ms < MIN_REFRESH_INTERVAL {
        return Err(NetwatchError::Config(format!(
            "Refresh interval too small (minimum {MIN_REFRESH_INTERVAL} ms)"
        )));
    }

    if interval_ms > MAX_REFRESH_INTERVAL {
        return Err(NetwatchError::Config(format!(
            "Refresh interval too large (maximum {MAX_REFRESH_INTERVAL} ms)"
        )));
    }

    Ok(())
}

/// Validates bandwidth values to prevent overflow and unrealistic values
///
/// # Security Considerations
/// - Prevents integer overflow in calculations
/// - Ensures reasonable bounds for bandwidth values
/// - Blocks obviously malicious or unrealistic inputs
pub fn validate_bandwidth(bandwidth_kbps: u64) -> Result<()> {
    // Maximum reasonable bandwidth: 1 Tbps = 1,000,000,000 kbps
    const MAX_BANDWIDTH: u64 = 1_000_000_000;

    if bandwidth_kbps > MAX_BANDWIDTH {
        return Err(NetwatchError::Config(format!(
            "Bandwidth value too large (maximum {MAX_BANDWIDTH} kbps)"
        )));
    }

    Ok(())
}

/// Validates configuration strings for injection attacks
///
/// # Security Considerations
/// - Prevents command injection through config values
/// - Blocks script injection attempts
/// - Validates string content and encoding
pub fn validate_config_string(value: &str, field_name: &str) -> Result<()> {
    if value.len() > 1024 {
        return Err(NetwatchError::Config(format!(
            "Configuration value too long for field: {field_name}"
        )));
    }

    // Check for null bytes and dangerous control characters
    if value.contains('\0')
        || value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(NetwatchError::Config(format!(
            "Invalid characters in configuration field: {field_name}"
        )));
    }

    // Block common injection patterns
    let dangerous_patterns = ["$(", "`", "${", "&&", "||", ";", "|", ">", "<", "&"];

    for pattern in &dangerous_patterns {
        if value.contains(pattern) {
            return Err(NetwatchError::Config(format!(
                "Suspicious pattern detected in field: {field_name}"
            )));
        }
    }

    Ok(())
}

/// Sanitizes user input by removing or escaping dangerous characters
///
/// # Security Considerations
/// - Removes null bytes and control characters
/// - Escapes shell metacharacters
/// - Truncates overly long inputs
pub fn sanitize_user_input(input: &str, max_length: usize) -> String {
    input
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .take(max_length)
        .collect::<String>()
        .replace('$', "\\$")
        .replace('`', "\\`")
        .replace('"', "\\\"")
        .replace('\'', "\\'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_name_validation() {
        // Valid interface names
        assert!(validate_interface_name("eth0").is_ok());
        assert!(validate_interface_name("wlan0").is_ok());
        assert!(validate_interface_name("en0").is_ok());
        assert!(validate_interface_name("lo").is_ok());
        assert!(validate_interface_name("br-docker0").is_ok());

        // Invalid interface names
        assert!(validate_interface_name("").is_err());
        assert!(validate_interface_name("../../../etc/passwd").is_err());
        assert!(validate_interface_name(
            "interface_with_very_long_name_that_exceeds_the_maximum_allowed_length"
        )
        .is_err());
        assert!(validate_interface_name("interface with spaces").is_err());
        assert!(validate_interface_name("interface\x00null").is_err());
        assert!(validate_interface_name("interface\nwith\nnewlines").is_err());
        assert!(validate_interface_name("/proc/net/dev").is_err());
        assert!(validate_interface_name("proc").is_err());
        assert!(validate_interface_name("sys").is_err());
    }

    #[test]
    fn test_file_path_validation() {
        // Valid file paths
        assert!(validate_file_path("/tmp/netwatch.log", Some("log")).is_ok());
        assert!(validate_file_path("/home/user/config.toml", Some("toml")).is_ok());
        assert!(validate_file_path("./local.log", Some("log")).is_ok());

        // Invalid file paths
        assert!(validate_file_path("", None).is_err());
        assert!(validate_file_path("../../../etc/passwd", None).is_err());
        assert!(validate_file_path("/etc/shadow", None).is_err());
        assert!(validate_file_path("/proc/version", None).is_err());
        assert!(validate_file_path("file\x00with\x00nulls", None).is_err());
        assert!(validate_file_path("/tmp/file.txt", Some("log")).is_err()); // Wrong extension
    }

    #[test]
    fn test_refresh_interval_validation() {
        // Valid intervals
        assert!(validate_refresh_interval(500).is_ok());
        assert!(validate_refresh_interval(1000).is_ok());
        assert!(validate_refresh_interval(30000).is_ok());

        // Invalid intervals
        assert!(validate_refresh_interval(50).is_err()); // Too small
        assert!(validate_refresh_interval(120000).is_err()); // Too large
    }

    #[test]
    fn test_bandwidth_validation() {
        // Valid bandwidth values
        assert!(validate_bandwidth(1000).is_ok());
        assert!(validate_bandwidth(1000000).is_ok());
        assert!(validate_bandwidth(100000000).is_ok());

        // Invalid bandwidth values
        assert!(validate_bandwidth(u64::MAX).is_err()); // Too large
        assert!(validate_bandwidth(2_000_000_000).is_err()); // Unrealistic
    }

    #[test]
    fn test_config_string_validation() {
        // Valid config strings
        assert!(validate_config_string("normal_value", "test_field").is_ok());
        assert!(validate_config_string("value-with-hyphens", "test_field").is_ok());
        assert!(validate_config_string("value_with_underscores", "test_field").is_ok());

        // Invalid config strings
        assert!(validate_config_string("value$(echo hack)", "test_field").is_err());
        assert!(validate_config_string("value`whoami`", "test_field").is_err());
        assert!(validate_config_string("value && rm -rf /", "test_field").is_err());
        assert!(validate_config_string("value\x00null", "test_field").is_err());
        assert!(validate_config_string(&"x".repeat(2000), "test_field").is_err());
        // Too long
    }

    #[test]
    fn test_sanitize_user_input() {
        assert_eq!(sanitize_user_input("normal input", 100), "normal input");
        assert_eq!(sanitize_user_input("input$(echo)", 100), "input\\$(echo)");
        assert_eq!(
            sanitize_user_input("input`whoami`", 100),
            "input\\`whoami\\`"
        );
        assert_eq!(
            sanitize_user_input("input\"quoted\"", 100),
            "input\\\"quoted\\\""
        );
        assert_eq!(
            sanitize_user_input("very long input that exceeds limit", 10),
            "very long "
        );
        assert_eq!(sanitize_user_input("input\x00null", 100), "inputnull");
    }
}
