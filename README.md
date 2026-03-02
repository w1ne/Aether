# Aether

Open-source embedded debugger with gRPC API for programmatic hardware control.

## What it does

- **Debug ARM Cortex-M** via ST-Link, J-Link, or CMSIS-DAP probes
- **gRPC API** for programmatic control (halt, step, read memory, set breakpoints)
- **RTT logging** with 1MB/s throughput
- **DWARF symbols** for source-level debugging
- **SVD peripheral decoding** for register inspection
- **Rust type support** (Vec, Option, Result pretty-printing)

## Architecture

- `aether-core` - Probe communication, symbol resolution, RTT handling
- `aether-ui` - egui-based GUI
- `aether-agent-api` - gRPC server for remote control

## Quick Start

### Prerequisites
- Rust (latest stable)
- Linux: `libusb-1.0-0-dev`, `libudev-dev`
- Windows: May need [Zadig](https://zadig.akeo.ie/) for USB driver setup

### Build & Run

Aether is a workspace with several components. You can build them individually:

#### GUI Debugger (Recommended for manual use)
```bash
cargo run --package aether-ui
```

#### gRPC Agent (For CI/CD and automation)
```bash
# Start the daemon
cargo run --package aether-agent-api --bin aether-daemon

# Use the CLI to interact
cargo run --package aether-agent-api --bin aether-cli -- probe list
```

#### Firmware (Requires cross-compilation toolchain)
The firmware is excluded from the default workspace build to speed up environment setup.
```bash
cd aether-demo-fw
cargo build
```

The UI will auto-detect connected debug probes.

## Development

See [docs/](docs/) for:
- [Contributing](docs/CONTRIBUTING.md) - Development guide (Trunk-Based)
- [Build Instructions](docs/BUILD.md) - Building from source (Linux, Windows, iOS)
- [Release Process](docs/RELEASING.md) - Maintainer's guide for releases
- [Testing Strategy](docs/TEST_STRATEGY.md) - Test requirements and HIL plan
- [Hardware Setup](docs/HARDWARE_SETUP.md) - Physical test rig instructions
- [CLI Reference](docs/CLI.md) - Full command documentation
- [Use Cases](docs/USE_CASES.md) - Feature documentation

### Pre-commit hooks
```bash
pip install pre-commit
pre-commit install
```

Runs `rustfmt`, `clippy`, and `cargo check` on commit.

## License
Apache-2.0 OR MIT
