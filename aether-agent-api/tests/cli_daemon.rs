use std::process::{Command, Stdio};
use std::time::Duration;
use std::thread;

#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "aether-cli", "--", "--help"])
        .output()
        .expect("Failed to run aether-cli");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: aether-cli"));
}

#[test]
fn test_daemon_help() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "aether-daemon", "--", "--help"])
        .output()
        .expect("Failed to run aether-daemon");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: aether-daemon"));
}
