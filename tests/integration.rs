use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_flag() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("netwatch"))
        .stdout(predicate::str::contains("network traffic monitor"));
}

#[test]
fn test_version_flag() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("netwatch"));
}

#[test]
fn test_list_flag() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();
    cmd.arg("--list").assert().success().stdout(
        predicate::str::contains("gif0")
            .or(predicate::str::contains("en0"))
            .or(predicate::str::contains("eth0")),
    );
}

#[test]
fn test_invalid_argument() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();
    cmd.arg("--invalid-flag")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error:"));
}

#[test]
fn test_invalid_interface() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();
    cmd.arg("nonexistent_interface_12345")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error").or(predicate::str::contains("Failed")));
}

#[test]
fn test_refresh_interval_validation() {
    // Test valid refresh interval
    let mut cmd = Command::cargo_bin("netwatch").unwrap();
    cmd.args(["-t", "500"]) // Valid refresh interval (>=100ms)
        .arg("--list")
        .assert()
        .success();
    
    // Test invalid refresh interval (too low - should fail due to security validation)
    let mut cmd = Command::cargo_bin("netwatch").unwrap();
    cmd.args(["-t", "50"]) // Too low refresh interval
        .arg("--list")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Refresh interval too small"));
}

#[test]
fn test_average_window_validation() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();
    cmd.args(["-a", "600"]) // Valid average window
        .arg("--list")
        .assert()
        .success();
}

#[test]
fn test_unit_format_validation() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();
    cmd.args(["-u", "k"]).arg("--list").assert().success();

    let mut cmd = Command::cargo_bin("netwatch").unwrap();
    cmd.args(["-u", "invalid"]).arg("--list").assert().failure();
}

#[test]
fn test_multiple_device_flag() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();
    cmd.arg("-m").arg("--list").assert().success();
}

#[test]
fn test_bandwidth_scale_options() {
    let mut cmd = Command::cargo_bin("netwatch").unwrap();
    cmd.args(["-i", "1000", "-o", "1000"])
        .arg("--list")
        .assert()
        .success();
}
