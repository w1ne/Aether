# Changelog

All notable changes to the Aether Debugger project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-13

### Added

#### Phase 11.1: Professional RTOS Diagnostics
- Real-time **FreeRTOS Stack Usage Analysis** for active tasks.
- **High-Water Mark (Peak) Detection** via pattern scanning (0xA5) for peak memory safety.
- **TaskSwitch Event Engine** broadcasting context switches at 20Hz for high-performance timeline views.
- Backend infrastructure for task-aware execution monitoring.

#### Phase 10 & 11: Robustness & Professional Stress
- Massive **64KB Memory Integrity** stress loop for data consistency verification.
- Malformed protocol **Fuzzing** (SVD XML and DWARF Symbol table corruption).
- Hardware **Interruption Recovery** logic for probe disconnects during heavy operations.
- Concurrent Agent Stress simulation verifying session thread-safety.
- Edge cases: Breakpoint limits, SWO baud mismatch, RTT buffer wrap handling.

#### Phase 9: Comprehensive Behavioral Verification
- **37 Verified E2E Scenarios** covering 100% of documented product use cases.
- Automated regression suite in `aether-core/tests/e2e_scenarios.rs`.
- Narrative documentation of all scenarios in `docs/USE_CASES.md`.

#### Milestone 9: Premium UI Overhaul
- Cyber-Industrial **Midnight Theme** using custom `egui` visuals.
- Balanced **3-pane layout** for professional workspace management.
- Micro-animations and polished status indicators for target state transitions.

#### Milestone 5: Breakpoints & Advanced Control
- `BreakpointManager` in `aether-core` for managing hardware breakpoints.
- Interactive breakpoint markers (●/○) in Disassembly View for toggling.
- Dedicated `BreakpointsView` panel in `aether-ui`.
- "Run to Cursor" (⏩) feature in disassembly for fast execution to a specific address.

#### Milestone 4: Memory View & Disassembly
- `MemoryManager` in `aether-core` for optimized block reads and writes.
- Hex Dump UI in `aether-ui` with ASCII preview and address navigation.
- `DisassemblyManager` integration using the `capstone` engine.
- ARM (Thumb) and RISC-V disassembly support.
- Live-updating disassembly view synced with the Program Counter (PC).

#### Milestone 3: Core Control & Register View
- `DebugManager` for target control (Halt, Resume, Step).
- `SessionHandle` background thread for non-blocking debug operations.
- `RegistersView` displaying R0-R15/PC/SP in a grid layout.
- Real-time core status reporting and error handling.

#### Milestone 2: Flash Programming
- `FlashManager` supporting ELF and binary file programming.
- `MpscFlashProgress` for thread-safe progress reporting from backend to UI.
- UI progress bar and status messages for Erasing/Programming/Verifying.

#### Milestone 1: Probe & Connection Foundation
- `ProbeManager` for scanning and connecting to debug probes via `probe-rs`.
- Target auto-detection and resource mapping (Flash/RAM).
- Initial `aether-ui` dashboard for probe selection and target info.

### Changed
- Refactored `AetherApp` state for better modularity between views.
- Optimized UI repaint frequency for 60 FPS while debugging.

### Fixed
- Resolved `probe-rs` v0.24 API breaking changes.
- Fixed duplicate dependency issues in `Cargo.toml`.
- Improved probe ownership transfer during flashing cycles.
