//! Security integration tests for Netwatch
//!
//! Tests that verify security properties and defensive behaviors

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_interface_name_injection_protection() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();

    // Test path traversal attempt (devices are positional arguments)
    cmd.arg("../etc/passwd")
        .arg("--test")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Invalid characters in interface name",
        ));
}

#[test]
fn test_log_file_path_validation() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();

    // Test path traversal in log file
    cmd.arg("--file")
        .arg("../../../etc/shadow")
        .arg("--test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Path traversal detected"));
}

#[test]
fn test_refresh_interval_bounds() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();

    // Test too small interval (DoS prevention)
    cmd.arg("--interval")
        .arg("50")
        .arg("--test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Refresh interval too small"));

    let mut cmd = Command::cargo_bin("netwatch").unwrap();

    // Test too large interval
    cmd.arg("--interval")
        .arg("120000")
        .arg("--test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Refresh interval too large"));
}

#[test]
fn test_bandwidth_value_validation() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();

    // Test unrealistic bandwidth value
    cmd.arg("--incoming")
        .arg("9999999999")
        .arg("--test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Bandwidth value too large"));
}

#[test]
fn test_no_privilege_escalation() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();

    // Should work without special privileges
    cmd.arg("--list").assert().success();
}

#[test]
fn test_config_file_security() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.log");

    // Create a test log file
    fs::write(&config_path, "test log content").unwrap();

    // Verify we can use the log file
    let mut cmd = Command::cargo_bin("netwatch").unwrap();
    cmd.arg("--file")
        .arg(config_path.to_str().unwrap())
        .arg("--test")
        .assert()
        .success();
}

#[test]
fn test_control_character_injection_protection() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();

    // Test control characters in interface name (using newline instead of null)
    cmd.arg("eth\npwd")
        .arg("--test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Control characters not allowed"));
}

#[test]
fn test_command_injection_protection() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();

    // Test command injection attempt in interface name
    cmd.arg("eth0; rm -rf /")
        .arg("--test")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Invalid characters in interface name",
        ));
}

#[test]
fn test_suspicious_interface_patterns() {
    let suspicious_names = vec!["proc", "sys", "dev"];

    for name in suspicious_names {
        let mut cmd = Command::cargo_bin("netwatch").unwrap();
        cmd.arg(name)
            .arg("--test")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "Suspicious interface name pattern",
            ));
    }
}

#[test]
fn test_log_file_extension_validation() {
    let temp_dir = TempDir::new().unwrap();
    let wrong_ext_path = temp_dir.path().join("netwatch.txt");

    fs::write(&wrong_ext_path, "test content").unwrap();

    let mut cmd = Command::cargo_bin("netwatch").unwrap();
    cmd.arg("--file")
        .arg(wrong_ext_path.to_str().unwrap())
        .arg("--test")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Invalid file extension, expected: log",
        ));
}

#[test]
fn test_stdout_logging_bypass() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();

    // Stdout logging should work (bypass validation)
    cmd.arg("--file").arg("-").arg("--test").assert().success();
}
