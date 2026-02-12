# Aether: Fast, Reliable Embedded Debugging

Aether is an open-source embedded debugger built to rival Segger Ozone. Its primary function is to serve as an **Agent Interface**—a hub connecting hardware to AI agents.

## Why Aether?
- **Agent Interface** - Expose hardware to AI agents via simple API
- **10x Faster than GDB** - <50ms step latency vs 100-500ms
- **Zero Config** - Auto-detects probes and chips, just works
- **Universal** - ST-Link, J-Link, CMSIS-DAP all supported
- **Free** - No $2000 license required
- **Rust-First** - Native understanding of Rust types

## Core Features
- **Agent API** - Control hardware programmatically
- **Fast Stepping** - <50ms latency, 60 FPS UI
- **RTT Logging** - 1MB/s+ non-intrusive logging
- **Memory/Register Views** - Live updates, SVD peripheral decoding
- **Source Debugging** - DWARF symbols, breakpoints, call stacks
- **Rust Types** - Pretty-print Vec, Option, Result

## 🏗️ Architecture
- **aether-core**: High-performance backend handling probe communication and symbol mapping.
- **aether-ui**: GPU-accelerated immediate-mode GUI (`egui`) for 60 FPS responsiveness.

## 🛠️ Getting Started

### Prerequisites
- [Rust](https://rustup.rs/) (latest stable)
- **Linux**: `libusb-1.0-0-dev` and `libudev-dev`
- **Windows**: [Zadig](https://zadig.akeo.ie/) may be needed for some probes (handled automatically in future versions).

### Build & Run
```bash
# Clone the repository
git clone https://github.com/aether-debugger/aether.git
cd aether

# Build the project
cargo build

# Run the UI
cargo run --package aether-ui
```

## 🛠️ Development

We maintain a high bar for code quality. Please refer to our internal documentation before contributing:

- **[Git Flow](docs/GIT_FLOW.md)**: Our branching and PR strategy.
- **[Testing Strategy](docs/TEST_STRATEGY.md)**: How we test Aether.
- **[Use Case Scenarios](docs/USE_CASES.md)**: Comprehensive guide to Aether's functionalities and workflows.

### Local Quality Control

To ensure your code meets our standards, we use `pre-commit` hooks.

1. **Install pre-commit**: `pip install pre-commit`
2. **Install hooks**: `pre-commit install`

Hooks will automatically run `rustfmt`, `clippy`, and `cargo check` on every commit.

## 📄 License
Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
