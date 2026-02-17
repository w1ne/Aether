use std::process::{Command, Stdio};
use std::time::Duration;
use std::thread;

fn wait_for_port(port: u16) -> bool {
    let addr = format!("127.0.0.1:{}", port);
    for _ in 0..50 {
        if std::net::TcpStream::connect(&addr).is_ok() {
            return true;
        }
        thread::sleep(Duration::from_millis(100));
    }
    false
}

#[test]
fn test_cli_daemon_integration() {
    let port = 50052;
    
    // 1. Start Daemon in MOCK mode
    let mut daemon = Command::new("cargo")
        .args(&["run", "--bin", "aether-daemon", "--", "--mock", "--port", &port.to_string()])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start daemon");

    // Wait for daemon to start
    assert!(wait_for_port(port), "Daemon did not start on port {}", port);

    // 2. Run CLI commands against it
    let url = format!("http://127.0.0.1:{}", port);

    // Test Halt
    let output = Command::new("cargo")
        .args(&["run", "--bin", "aether-cli", "--", "--url", &url, "halt"])
        .output()
        .expect("Failed to run CLI halt");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Halted"), "CLI failed to halt: {}", stdout);

    // Test Resume
    let output = Command::new("cargo")
        .args(&["run", "--bin", "aether-cli", "--", "--url", &url, "resume"])
        .output()
        .expect("Failed to run CLI resume");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Resumed"), "CLI failed to resume: {}", stdout);

    // Test Read Register
    let output = Command::new("cargo")
        .args(&["run", "--bin", "aether-cli", "--", "--url", &url, "regs", "--num", "15"])
        .output()
        .expect("Failed to run CLI regs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Mock returns 0xFACEFEED
    assert!(stdout.contains("FACEFEED"), "CLI failed to read reg: {}", stdout);

    // 3. Kill Daemon
    let _ = daemon.kill();
}

