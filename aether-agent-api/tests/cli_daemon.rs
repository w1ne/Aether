#![allow(clippy::zombie_processes)]
//! Integration tests for the Aether CLI and Daemon.

use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

struct DaemonHandle {
    child: std::process::Child,
}

impl Drop for DaemonHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn wait_for_port(port: u16) -> bool {
    let addr = format!("127.0.0.1:{port}");
    // Wait up to 30 seconds (300 * 100ms)
    for i in 0..300 {
        if std::net::TcpStream::connect(&addr).is_ok() {
            return true;
        }
        if i % 20 == 0 {
            println!("Waiting for port {port}... ({}s)", i / 10);
        }
        thread::sleep(Duration::from_millis(100));
    }
    false
}

fn run_cli(url: &str, args: &[&str]) -> (String, String) {
    let mut full_args = vec!["run", "--bin", "aether-cli", "--", "--url", url];
    full_args.extend_from_slice(args);

    let output = Command::new("cargo").args(&full_args).output().expect("Failed to run CLI");

    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn test_cli_daemon_integration() {
    let port = 50059; // Use a distinct port for this test

    // 1. Start Daemon in MOCK mode
    let daemon_exe = env!("CARGO_BIN_EXE_aether-daemon");
    let daemon_child = Command::new(daemon_exe)
        .args(["--mock", "--port", &port.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start daemon");

    // Wait for daemon to start with a longer timeout (30s)
    if !wait_for_port(port) {
        // If it failed, try to get some diagnostics
        let output = daemon_child.wait_with_output().ok();
        if let Some(out) = output {
            eprintln!("Daemon failed to start. Port: {port}");
            eprintln!("Stdout: {}", String::from_utf8_lossy(&out.stdout));
            eprintln!("Stderr: {}", String::from_utf8_lossy(&out.stderr));
        } else {
            eprintln!("Daemon failed to start and could not capture output.");
        }
        panic!("Daemon did not start on port {port} after 30s");
    }

    let _daemon = DaemonHandle { child: daemon_child };

    let url = format!("http://127.0.0.1:{port}");

    // Test Core Halt
    let (stdout, stderr) = run_cli(&url, &["core", "halt"]);
    assert!(stdout.contains("Halted"), "STDOUT: {stdout}\nSTDERR: {stderr}");

    // Test Core Resume
    let (stdout, stderr) = run_cli(&url, &["core", "resume"]);
    assert!(stdout.contains("Resumed"), "STDOUT: {stdout}\nSTDERR: {stderr}");

    // Test Core Regs
    let (stdout, stderr) = run_cli(&url, &["core", "regs", "--num", "15"]);
    assert!(stdout.contains("FACEFEED"), "STDOUT: {stdout}\nSTDERR: {stderr}");

    // Test Target Disassemble
    let (stdout, stderr) = run_cli(&url, &["target", "disasm", "0x08000000", "2"]);
    assert!(stdout.contains("mov  r0, r1"), "STDOUT: {stdout}\nSTDERR: {stderr}");

    // Test Target Flash
    let (stdout, stderr) = run_cli(&url, &["target", "flash", "dummy.elf"]);
    assert!(stdout.contains("Flash Complete"), "STDOUT: {stdout}\nSTDERR: {stderr}");

    // Test Target LoadSvd
    let (stdout, stderr) = run_cli(&url, &["target", "load-svd", "dummy.svd"]);
    assert!(stdout.contains("SVD Loaded"), "STDOUT: {stdout}\nSTDERR: {stderr}");
}
