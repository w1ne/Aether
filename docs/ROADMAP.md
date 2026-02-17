# Aether Debugger Roadmap

**Mission**: Build the ultimate **Agent Interface** for embedded systems. Serve as the bridge between AI agents and hardware.

## Core Principles
- **Fast** - <50ms step latency (10x better than GDB)
- **Reliable** - Zero crashes, auto-reconnect
- **Zero Config** - Plug in probe, start debugging
- **Universal** - Works with ST-Link, J-Link, CMSIS-DAP
- **Open Source** - No $2000 license

## Competitive Target

| Feature | Segger Ozone | GDB + OpenOCD | **Aether Target** |
|---------|--------------|---------------|-------------------|
| Step Latency | <10ms | 100-500ms | **<50ms** |
| RTT Throughput | 2MB/s | Limited | **2MB/s** |
| Setup Time | 30 seconds | 30 minutes | **30 seconds** |
| Probe Support | J-Link only | Universal (buggy) | **Universal (solid)** |
| Cost | $200-2000 | Free | **Free** |
| Rust Types | Basic | Basic | **Native** |

---

## Phase 1: Core Debugger (Months 1-3) - **COMPLETED**

**Goal**: Match Ozone for basic debugging on STM32/ESP32/nRF52.

### Features
- **Probe Detection**: Auto-detect ST-Link, J-Link, CMSIS-DAP
- **Flash Programming**: CMSIS-Pack flash algorithms
- **Core Control**: Halt, Resume, Step, Reset
- **Breakpoints**: Hardware (4-8) + Software (unlimited)
- **Memory View**: Hex editor with live updates
- **Register View**: Core registers + SVD peripheral decoding
- **RTT Terminal**: Multi-channel (0-15), 1MB/s+
- **Disassembly**: ARM Thumb/Thumb2, RISC-V, Xtensa
- **Source Debug**: DWARF symbol parsing, source-level stepping

### Performance Targets
- Step latency: <50ms
- RTT throughput: >1MB/s
- UI frame rate: 60 FPS
- Crash rate: <0.1%

### Deliverables
- `aether` v0.1.0 binaries (Windows, macOS, Linux)
- Support for STM32F4, ESP32, nRF52
- Quick start guide + 3 example projects
- Benchmark: Faster than GDB, competitive with Ozone

**Exit Criteria**: Can debug common boards as fast as Ozone, costs $0.

---

## Phase 2: Production Ready (Months 4-6)

**Goal**: Daily-driveable for professional embedded engineers.

### Features
- **Expanded Hardware**: 50+ chip families (STM32, ESP32, nRF, RP2040, etc.)
- **Variable Watch**: Evaluate expressions, inspect structs
- **Call Stack**: Unwind with DWARF debug info
- **Rust Types**: Pretty-print Vec, Option, Result, custom enums
- **Session Export**: Save/load debugging sessions
- **Stability**: Auto-reconnect on probe disconnect
- **Error Messages**: Actionable solutions for common issues

### Performance Targets
- Zero critical bugs
- 80%+ test coverage
- Cross-platform CI passing

### Deliverables
- `aether` v0.5.0 (production beta)
- Hardware compatibility matrix (50+ chips)
- VS Code extension (basic DAP adapter)
- User manual + troubleshooting guide

**Exit Criteria**: Professionals can use it for daily work without reverting to Ozone.

---

## Phase 2.5: The Agent Interface (Months 6-7)

**Goal**: Transform Aether into a "Headless" platform for AI Agents.

### Features
- **Client-Server Architecture**: Decouple UI from Core (gRPC).
- **Agent API**: Full programmatic control (Halt, Step, Read Mem) via Protobuf.
- **Event Stream**: Real-time structured events (no polling).
- **Headless Mode**: Run `aether-core` without GUI for CI/Agents.
- **Agent Chat**: UI tab to communicate with connected agents.

### Deliverables
- `aether-core` daemon mode
- `aether-agent-api` crate on crates.io
- Python/TypeScript client SDKs for agents
- "Ghost Mode" demo (Agent debugging while human watches)

---

## Phase 3: Rust & RTOS Excellence (Months 7-12)

**Goal**: Best debugger for Rust embedded and RTOS development.

### Features
- **Timeline View**: Simple execution timeline (not Perfetto)
- **RTOS Tasks**: Visualize FreeRTOS, Zephyr, Embassy tasks
- **Rust Async**: Task tree for Embassy/RTIC executors
- **Remote Debug**: Simple WebSocket for remote hardware
- **Advanced RTT**: Binary protocol parsing, custom formatters
- **Oscilloscope**: Plot variables over time (1Hz-10kHz)

### Deliverables
- `aether` v1.0.0 (stable release)
- Best-in-class Rust debugging experience
- RTOS task visualization
- Remote debugging for distributed teams

**Exit Criteria**: Rust embedded developers prefer Aether over all alternatives.

---

## Phase 4+: User-Driven

**Don't plan 2 years ahead. Build what users request.**

Potential features (only if users ask):
- Plugin system (WASM)
- More RTOS support
- Trace analysis improvements
- Enterprise features (if someone pays)

**Never build**: Multiplayer, SQL queries, AI features, certification programs, localization (unless proven demand).

---

## Success Metrics

### Technical
- Step latency: <50ms (measured)
- RTT throughput: >1MB/s (measured)
- UI responsiveness: 60 FPS sustained
- Crash rate: <0.1% of sessions
- Test coverage: >80%

### Adoption
- Month 3: 100 users
- Month 6: 1000 users
- Month 12: 5000 users
- GitHub stars: 500 → 5000 → 20000

### Quality
- Zero critical bugs in stable releases
- Issues triaged within 48 hours
- CI passes on all platforms

---

## What We're NOT Building

- ❌ Multiplayer debugging
- ❌ Perfetto SQL queries
- ❌ WASM plugin marketplace (Phase 1-3)
- ❌ AI-powered features
- ❌ ISO 26262 compliance tools
- ❌ Educational certification programs
- ❌ Localization (English only for now)
- ❌ "Observability Platform" complexity

**Focus**: Fast, reliable debugger. Nothing more.

---

## Technology Stack

- **Core**: Rust + probe-rs 0.24+
- **UI**: egui 0.28+ (60 FPS immediate-mode)
- **Parsing**: gimli (DWARF), capstone (disassembly)
- **Build**: Cargo workspace, strict lints
- **CI**: GitHub Actions (Windows, macOS, Linux)

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| probe-rs API gaps | Contribute upstream, fork if needed |
| Performance regression | Automated benchmarking in CI |
| Hardware compatibility | Test with real boards weekly |
| User adoption | Ship early, iterate fast |

---

## The Bottom Line

Engineers don't need an "Observability Platform."

They need a debugger that:
1. Works immediately (zero config)
2. Is fast (<50ms step latency)
3. Doesn't crash
4. Shows memory/registers clearly
5. Handles RTT logging
6. Costs $0 instead of $2000

**That's what we're building.**
