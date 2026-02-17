# Aether Agent API

gRPC-based API for programmatic control of the Aether debugger.

## Quick Start

### 1. Start the Aether UI
```bash
cargo run --bin aether-ui
```

Connect to your debug probe and target device through the UI.

### 2. Run the Test Client
In a separate terminal:
```bash
cargo run --example client --package aether-agent-api
```

## API Overview

The Agent API exposes the following gRPC methods:

- `Halt()` - Halt the target core
- `Resume()` - Resume execution
- `Step()` - Single-step execution
- `GetStatus()` - Get current core status
- `SubscribeEvents()` - Stream debug events (halted, resumed, memory, registers)

## Architecture

The gRPC server runs on port `50051` and is automatically started by `aether-ui` when a debug session is active. It uses `tokio::sync::broadcast` to fan out debug events to both the UI and any connected API clients.

## Example Client

See `examples/client.rs` for a complete example of connecting to the API and sending commands.

## Protocol Definition

The full protocol is defined in `proto/aether.proto`.
