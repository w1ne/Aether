## [0.2.4] - 2026-03-02

### Fixed
- **iOS CI Cross-Compilation**: Fixed a linker error during CI where `aether-ui` was attempting to build as an iOS executable rather than a static library by adding a explicit `[lib]` crate type. Disabled `onig_sys` compilation in `syntect` by switching to the pure-Rust `fancy-regex` to prevent Xcode SDK conflicts on the GitHub Actions macOS runner.

## [0.2.3] - 2026-03-02

### Fixed
- **iOS Build Consistency**: Implemented comprehensive hardware modularization across `aether-core`, `aether-agent-api`, and `aether-ui`. The stack now builds successfully for iOS by disabling hardware-specific dependencies (`probe-rs`, `hidapi`) which were causing compilation failures on unsupported target OS.

## [0.2.2] - 2026-03-02

### Added
- **Enhanced Agent Guidelines**: Updated `AGENTS.md` with strict delivery standards, a visual proof protocol, and comprehensive hardware safety rules.
- **Improved Release Workflow**:
  - Added native **macOS Desktop** build support.
  - Fixed missing Linux hardware dependencies (`libusb`, `libudev`).
  - Resolved artifact naming collisions in GitHub Releases.

## [0.2.1] - 2026-03-02

### Added
- **Semihosting & ITM/SWV**: Full gRPC and CLI support for ARM Semihosting and ITM/SWV protocol streams.
- **Headless Mode**: `aether-daemon` for background execution and `aether-cli` for terminal-based control.
- **Improved HIL Testing**: New regression tests for real-hardware verification and automated flash programming checks.
- **UI Enhancements**:
  - Overhauled view icons with high-unicode symbols (`⫘`, `✍`, `📈`, `⛃`, `🔎`, `🖴`, `☷`).
  - Improved font fallback for Linux environments.
  - Tab recovery mechanism for closed views.
- **Release Strategy**: Automated CI/CD for Linux, Windows, and iOS via GitHub Actions.
- **Build Documentation**: Added `docs/BUILD.md` and `docs/RELEASING.md`.

### Fixed
- Initial attempt at making hardware dependencies optional for iOS. (Superseded by robust modularization in 0.2.3).
- Non-exhaustive match in agent-api for session handle types.
- Flash programming progress reporting and reliability on varied board resets.
- Dependency synchronization across `aether-core` and `aether-agent-api`.

### Removed
- Experimental Agent Chat interface from UI to focus on core gRPC stream interactions.

## [0.2.0] - 2026-02-27

### Added
- **SVD Peripheral Integration**: `SvdManager` for parsing SVD files and extracting peripheral/register info.
- **Peripheral View**: Dedicated 5-column layout for live hardware inspection and bitfield decoding.
- **Field Editing**: Interactive editing of peripheral register fields with write-back support.
- **FreeRTOS Diagnostics**: Stack usage analysis and peak memory detection.
- **High-Speed Plotting**: Optimized 10kHz data stream visualization.

## [0.1.0] - 2026-02-13

### Added
- **Probe & Connection Foundation**: Core logic for target auto-detection and probe management.
- **Flash Programming**: Multi-format ELF/binary support with thread-safe progress reporting.
- **Memory & Disassembly**: Professional hex dump and integrated `capstone` disassembly.
- **Breakpoint Management**: Hardware breakpoint support with interactive UI markers.
