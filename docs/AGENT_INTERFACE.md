# Aether Agent Interface Specification

## Overview
The Aether Agent Interface allows AI agents and external tools to programmatically control the debugger, inspect state, and receive real-time events. It is implemented as a **gRPC service** exposed by the Aether Core.

## Architecture
- **Protocol**: gRPC (HTTP/2) with Protobuf serialization.
- **Transport**: TCP (default port `50051`).
- **Model**: Bidirectional streaming. Agents send commands (RPCs) and subscribe to a continuous stream of `DebugEvent`s.

## Service Definition (`AetherDebug`)

### 1. Execution Control
Standard debugger controls. **Most commands are synchronous**: they return only after the operation completes (e.g., `Step` returns after the step is done and the core halts again). `Resume` is asynchronous.

| RPC | Description | Behavior |
|---|---|---|
| `Halt` | Pause target execution. | **Synchronous**: Returns after `HaltedEvent`. |
| `Resume` | Resume target execution. | **Asynchronous**: Returns immediately. |
| `Step` | Execute one instruction. | **Synchronous**: Returns after `HaltedEvent`. |
| `StepOver` | Step over the current line/call. | **Synchronous**: Returns after `HaltedEvent`. |
| `StepInto` | Step into the function call. | **Synchronous**: Returns after `HaltedEvent`. |
| `StepOut` | Run until the current function returns. | **Synchronous**: Returns after `HaltedEvent`. |
| `Reset` | Reset the target MCU. | **Synchronous**: Returns after `HaltedEvent` (at reset vector). |

### 2. Breakpoints
Manage hardware and software breakpoints.

| RPC | Input | Description |
|---|---|---|
| `SetBreakpoint` | `address: uint64` | Set a breakpoint at the specified address. |
| `ClearBreakpoint` | `address: uint64` | Remove a breakpoint from the specified address. |
| `ListBreakpoints` | `Empty` | Returns a list of all active breakpoint addresses. |

### 3. State Inspection
Read internal chip state.

| RPC | Input | Returns |
|---|---|---|
| `ReadMemory` | `address, length` | Raw bytes from memory. |
| `ReadRegister` | `reg_num` | 64-bit register value. |
| `GetStatus` | `Empty` | Core status (Halted/Running, PC). |
| `GetStack` | `Empty` | Current call stack frames (PC, Function, File, Line). |
| `GetTasks` | `Empty` | RTOS task list (Name, State, Stack Usage). |
| `ReadPeripheral` | `perp, reg` | SVD-decoded register value. |

### 4. State Mutation
Modify chip state.

| RPC | Input | Description |
|---|---|---|
| `WriteMemory` | `address, data` | Write bytes to memory. |
| `WriteRegister` | `reg_num, value` | Write to a core register. |
| `WritePeripheral` | `perp, reg, field, val` | Write to a named peripheral field (SVD). |
| `RttWrite` | `channel, data` | Send data to the target via RTT. |

## Events (`DebugEvent`)
Agents should `SubscribeEvents` immediately upon connection.

### `HaltedEvent`
Triggered when the core stops (breakpoint, user halt, fault).
```proto
message HaltedEvent {
    uint64 pc = 1;
}
```

### `RttEvent`
Stream of stdout/log data from the target.
```proto
message RttEvent {
    uint32 channel = 1;
    bytes data = 2; // UTF-8 text or binary
}
```

### `PlotEvent`
Real-time data for visualization (e.g., motor speed, temperature).
```proto
message PlotEvent {
    string name = 1;
    double timestamp = 2;
    double value = 3;
}
```

## Example Usage (Python)

```python
import grpc
import aether_pb2
import aether_pb2_grpc

channel = grpc.insecure_channel('localhost:50051')
stub = aether_pb2_grpc.AetherDebugStub(channel)

# 1. Subscribe to events
events = stub.SubscribeEvents(aether_pb2.Empty())

# 2. Halt and Inspect
stub.Halt(aether_pb2.Empty())
# Sync call: We are now halted.

# 3. Read R0
reg = stub.ReadRegister(aether_pb2.ReadRegisterRequest(register_number=0))
print(f"R0: {hex(reg.value)}")

# 4. Resume
stub.Resume(aether_pb2.Empty())
# Now running...
```
