use std::process::{Command, Stdio};
use std::time::Duration;
use std::thread;

fn wait_for_port(port: u16) -> bool {
    let addr = format!("127.0.0.1:{}", port);
    for i in 0..100 {
        if std::net::TcpStream::connect(&addr).is_ok() {
            return true;
        }
        if i % 20 == 0 {
            println!("Waiting for port {}... ({}s)", port, i / 10);
        }
        thread::sleep(Duration::from_millis(100));
    }
    false
}

fn run_cli(url: &str, args: &[&str]) -> (String, String) {
    let mut full_args = vec!["run", "--bin", "aether-cli", "--", "--url", url];
    full_args.extend_from_slice(args);

    let output = Command::new("cargo")
        .args(&full_args)
        .output()
        .expect("Failed to run CLI");

    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn test_cli_daemon_integration() {
    let port = 50053; // Use different port to avoid conflicts

    // 1. Start Daemon in MOCK mode
    let mut daemon = Command::new("cargo")
        .args(&["run", "--bin", "aether-daemon", "--", "--mock", "--port", &port.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start daemon");

    // Wait for daemon to start
    assert!(wait_for_port(port), "Daemon did not start on port {}", port);

    let url = format!("http://127.0.0.1:{}", port);

    // Test Core Halt
    let (stdout, stderr) = run_cli(&url, &["core", "halt"]);
    assert!(stdout.contains("Halted"), "STDOUT: {}\nSTDERR: {}", stdout, stderr);

    // Test Core Resume
    let (stdout, stderr) = run_cli(&url, &["core", "resume"]);
    assert!(stdout.contains("Resumed"), "STDOUT: {}\nSTDERR: {}", stdout, stderr);

    // Test Core Regs
    let (stdout, stderr) = run_cli(&url, &["core", "regs", "--num", "15"]);
    assert!(stdout.contains("FACEFEED"), "STDOUT: {}\nSTDERR: {}", stdout, stderr);

    // Test Target Disassemble
    let (stdout, stderr) = run_cli(&url, &["target", "disasm", "0x08000000", "2"]);
    assert!(stdout.contains("mov  r0, r1"), "STDOUT: {}\nSTDERR: {}", stdout, stderr);

    // Test Target Flash
    let (stdout, stderr) = run_cli(&url, &["target", "flash", "dummy.elf"]);
    assert!(stdout.contains("Flash Complete"), "STDOUT: {}\nSTDERR: {}", stdout, stderr);

    // Test Target LoadSvd
    let (stdout, stderr) = run_cli(&url, &["target", "load-svd", "dummy.svd"]);
    assert!(stdout.contains("SVD Loaded"), "STDOUT: {}\nSTDERR: {}", stdout, stderr);

    // 3. Kill Daemon
    let _ = daemon.kill();
}
