#!/bin/bash
set -e

# Configuration
DAEMON_BIN=./target/release/aether-daemon
CLI_BIN=./target/release/aether-cli
DAEMON_PORT=50051
LOG_DIR=test_results
mkdir -p $LOG_DIR
LOG_FILE=$LOG_DIR/cli_logs.txt

# Parse args
MOCK_FLAG=""
if [[ "$1" == "--mock" ]]; then
    MOCK_FLAG="--mock"
    echo "Running in MOCK mode"
fi

# Cleanup function
cleanup() {
    echo "Stopping daemon..."
    kill $DAEMON_PID 2>/dev/null || true
}
trap cleanup EXIT

# 1. Start Daemon
echo "Starting Aether Daemon..." | tee -a $LOG_FILE
$DAEMON_BIN --port $DAEMON_PORT $MOCK_FLAG >> $LOG_FILE 2>&1 &
DAEMON_PID=$!
sleep 2 # Wait for startup

# 2. Run Tests
{
    echo "--- Starting CLI Tests ---"
    
    echo "[TEST] Status"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT status
    
    echo "[TEST] Reset"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT core reset
    sleep 1
    
    echo "[TEST] Halt"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT core halt
    
    echo "[TEST] Status (Expect Halted)"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT status
    
    echo "[TEST] Read Registers"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT core regs
    
    echo "[TEST] Step"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT core step
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT core regs --num 15 # Check PC
    
    echo "[TEST] Resume"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT core resume
    sleep 1
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT status
    
    echo "[TEST] Step Over"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT core step-over
    
    echo "[TEST] Write Register"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT core write-reg 0 0x1234
    
    echo "[TEST] Tasks"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT rtos tasks
    
    echo "[TEST] Load SVD"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT target load-svd /tmp/mock.svd
    
    echo "[TEST] Load Symbols"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT target load-symbols /tmp/mock.elf
    
    echo "[TEST] Flash (Simulated)"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT target flash /tmp/mock.bin
    
    echo "[TEST] Disassemble"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT target disasm 0x08000000 4
    
    echo "[TEST] Enable Semihosting"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT trace enable-semihosting
    
    echo "[TEST] Watch"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT rtos watch counter
    
    echo "[TEST] Peripheral Read"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT target read-peri RCC CR
    
    echo "[TEST] RTT Write"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT trace rtt-write 0 "Hello World"
    
    echo "--- Tests Completed Successfully ---"
} 2>&1 | tee -a $LOG_FILE
