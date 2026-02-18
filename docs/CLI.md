# Aether CLI Reference

The `aether-cli` is the command-line interface for interacting with the Aether Debugger. It communicates with the `aether-daemon` via gRPC.

Before using the CLI, you must start the `aether-daemon`.

### Zero-Config Startup (Recommended)
You can start the daemon without any flags. It will start the gRPC server and allow you to discover and attach to hardware later via the CLI or UI.

```bash
aether-daemon
```

### Discovery & Dynamic Attachment
Once the daemon is running, you can list available probes and attach to a target:

```bash
# List available probes
aether-cli probe list

# Attach to a target (uses probe 0 and auto-detection by default)
aether-cli probe attach --chip auto

# Attach to a specific chip
aether-cli probe attach --chip STM32L476RGTx
```

### Manual Configuration (Startup)
If you prefer to connect immediately on startup, you can specify your hardware explicitly:

* `--chip`: The exact target chip variant (e.g., `STM32L476RGTx`).
* `--protocol`: Force a protocol (`swd` or `jtag`).
* `--under-reset`: Connect while holding the target in reset.
* `--port`: gRPC server port (default: `50051`).
* `--probe-index`: Index of the probe to use if multiple are connected.

## Usage

```bash
aether-cli [OPTIONS] <COMMAND>
```

### Options
* `--url`: gRPC server URL (default: `http://[::1]:50051`)

## Command Structure

Commands are grouped into logical categories: `core`, `memory`, `target`, `rtos`, and `trace`.

### Core Commands
Operations related to the processor core state and registers.

* `core halt`: Stop CPU execution.
* `core resume`: Resume CPU execution.
* `core reset`: Reset the target device.
* `core step`: Execute a single instruction.
* `core step-over`: Step over function call.
* `core step-into`: Step into function call.
* `core step-out`: Step out of current function.
* `core regs [--num <N>]`: List all registers or read a specific one.
* `core write-reg <NUM> <HEX_VALUE>`: Write a value to a register.

### Memory Commands
Direct memory access.

* `memory read <ADDRESS> <LENGTH>`: Read a range of memory (hex output).
* `memory write <ADDRESS> <HEX_DATA>`: Write hex bytes to memory.

### Target Commands
Target-specific operations like flashing and symbols.

* `target flash <PATH>`: Flash an ELF or binary file to the device.
* `target load-svd <PATH>`: Load an SVD file to enable peripheral register decoding.
* `target load-symbols <PATH>`: Load ELF symbols for high-level debugging.
* `target disasm <ADDRESS> [COUNT]`: Disassemble instructions starting at an address.
* `target read-peri <PERIPHERAL> <REGISTER>`: Read a peripheral register value.
* `target write-peri <PERIPHERAL> <REGISTER> <FIELD> <HEX_VALUE>`: Write to a peripheral register field.
* `target breakpoints`: List all active hardware breakpoints.
* `target break <ADDRESS>`: Set a hardware breakpoint.
* `target clear <ADDRESS>`: Remove a breakpoint.

### RTOS Commands
High-level introspection for RTOS and variables.

* `rtos tasks`: List active RTOS tasks (priority, state, stack usage).
* `rtos stack`: Show the current call stack (function names, files, lines).
* `rtos watch <NAME>`: Watch a variable by name.

### Trace Commands
Real-time logging and tracing protocols.

* `trace rtt-write <CHANNEL> <STRING>`: Send data to an RTT channel (e.g., shell command input).
* `trace semihosting`: Enable ARM Semihosting output (stdout/stderr redirection).
* `trace itm [--baud <BAUD>]`: Enable Instrumentation Trace Macrocell (ITM) output via SWO pin.

### Global Commands
* `status`: Quick check of connection and core state.

## Examples

#### Flashing and Resetting
```bash
aether-cli target flash firmware.elf
aether-cli core reset
```

#### Inspecting Peripheral via SVD
```bash
aether-cli target load-svd STM32F405.svd
aether-cli target read-peri RCC CR
```

#### Real-time Logs (RTT)
```bash
aether-cli trace rtt-write 0 "Hello World"
```

## AI Agent Integration (OpenClaw)

`aether-cli` is designed to be easily wrapped by AI agents like **OpenClaw**. 

### Example Tool Definition
You can map CLI commands to agent tools. For example, in an `aether_tools.yaml` for OpenClaw:

```yaml
- name: aether_halt
  description: Stop CPU execution
  command: aether-cli core halt

- name: aether_status
  description: Get debugger status
  command: aether-cli status
```

See [integrations/openclaw/aether_tools.yaml](file:///home/andrii/Projects/AetherDebugger/integrations/openclaw/aether_tools.yaml) for a complete example.
