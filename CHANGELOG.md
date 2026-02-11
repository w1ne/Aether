# Changelog

All notable changes to the Aether Debugger project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
