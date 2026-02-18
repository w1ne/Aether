#!/bin/bash
set -e

# Configuration
CLI_BIN=./target/release/aether-cli
DAEMON_PORT=50051
LOG_DIR=test_results
mkdir -p $LOG_DIR
LOG_FILE=$LOG_DIR/hw_cli_logs.txt

# This script assumes the daemon is already running with correct hardware settings.

{
    echo "--- Starting Hardware CLI Tests ---"

    echo "[TEST] Status"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT status

    echo "[TEST] Halt"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT core halt

    echo "[TEST] Read Registers"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT core regs

    echo "[TEST] Step (Basic)"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT core step
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT core regs --num 15

    echo "[TEST] Write Register"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT core write-reg 0 0x1234

    echo "[TEST] Resume"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT core resume
    sleep 1
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT status

    echo "[TEST] Disassemble"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT target disasm 0x08000000 8

    echo "[TEST] Read Memory"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT core read-mem 0x08000000 16

    echo "[TEST] Reset"
    $CLI_BIN --url http://127.0.0.1:$DAEMON_PORT core reset

    echo "--- Hardware Tests Completed ---"
} 2>&1 | tee -a $LOG_FILE
