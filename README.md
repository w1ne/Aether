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
```bash
git clone https://github.com/aether-debugger/aether.git
cd aether
cargo run --package aether-ui
```

The UI will auto-detect connected debug probes.

## Development

See [docs/](docs/) for:
- [Git Flow](docs/GIT_FLOW.md) - Branching strategy
- [Testing Strategy](docs/TEST_STRATEGY.md) - Test requirements
- [Use Cases](docs/USE_CASES.md) - Feature documentation

### Pre-commit hooks
```bash
pip install pre-commit
pre-commit install
```

Runs `rustfmt`, `clippy`, and `cargo check` on commit.

## License
Apache-2.0 OR MIT
