## [0.2.6] - 2026-03-03

### Added
- **Multi-Architecture Support**:
  - **Windows**: Added 32-bit (x86) support alongside the existing 64-bit version.
  - **macOS**: Switched to a **Universal Binary** supporting both Apple Silicon (M1/M2/M3) and Intel-based Macs.
- **Improved Release Workflow**: Artifacts are now clearly labeled by architecture to simplify installation in diverse engineering environments.

## [0.2.5] - 2026-03-02

### Added
- **Desktop Focus**: Re-affirmed Aether as a desktop-first tool for Windows, macOS, and Linux.
- **WGPU Support for All Platforms**: Native WGPU rendering support across all desktop environments for improved performance and Metal/Vulkan compatibility.

### Changed
- **egui 0.33 Ecosystem Upgrade**: Major upgrade from `egui/eframe 0.28` to `0.33`. Includes updates to `egui_plot` (0.34) and `egui_dock` (0.18).
- **Core Portability**: Workspace crates (`aether-core`, `aether-agent-api`) now support a `hardware`-free build mode for simulation-only use.
- **Hardware Dependency Modularization**: Fully decoupled hardware-specific dependencies (`probe-rs`, `hidapi`) behind a `hardware` feature.

### Fixed
- **eframe Workspace Compilation**: Resolved a critical upstream bug in `eframe 0.28` that caused workspace compilation errors on some macOS SDKs.
- **CI Release Permissions**: Fixed GitHub Actions workflow permissions (`contents: write`) to allow automated release creation and asset uploading.
- **Syntect C-Dependency Resolution**: Migrated `syntect` to pure-Rust `fancy-regex` to prevent platform-specific C toolchain conflicts.


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
