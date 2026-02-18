#!/bin/bash
set -e

echo "--- Starting Aether HIL Smoke Test ---"

# Build latest binaries
echo "Building binaries..."
cargo build -p aether-agent-api --release

# Start daemon in background
echo "Starting aether-daemon..."
./target/release/aether-daemon > daemon.log 2>&1 &
DAEMON_PID=$!

# Ensure daemon is killed on exit
trap "echo 'Cleaning up...'; kill $DAEMON_PID || true; wait $DAEMON_PID 2>/dev/null || true" EXIT

# Wait for daemon to start
sleep 2

# 1. Attach to probe
echo "Attaching to probe..."
./target/release/aether-cli --url http://127.0.0.1:50051 probe attach --chip STM32L476RGTx

# 2. Halt the core
echo "Halting core..."
./target/release/aether-cli --url http://127.0.0.1:50051 core halt

# 3. Read status
echo "Checking status..."
./target/release/aether-cli --url http://127.0.0.1:50051 status

# 4. Step the core
echo "Stepping core..."
./target/release/aether-cli --url http://127.0.0.1:50051 core step

# 5. Resume the core
echo "Resuming core..."
./target/release/aether-cli --url http://127.0.0.1:50051 core resume

echo ""
echo "HIL Smoke Test PASSED!"
