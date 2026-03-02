## [0.2.5] - 2026-03-02

### Fixed
- **iOS artifact output**: Re-added `[lib]` section with `crate-type = ["staticlib"]` to `aether-ui` so `cargo build --target aarch64-apple-ios` produces `libaether_ui.a` as expected by the release workflow.

## [0.2.4] - 2026-03-02

### Changed
- **egui 0.33 upgrade**: Updated `eframe`/`egui` from `0.28` to `0.33`, `egui_plot` to `0.34`, and `egui_dock` to `0.18`. This resolves a hardcoded upstream bug that blocked the iOS native build (`ViewportId::ROOT` missing import in `eframe 0.28`). iOS static library is built via the `wgpu` backend with OpenGL/glutin disabled.

## [0.2.3] - 2026-03-02

### Fixed
- **iOS CI Cross-Compilation & Hardware Modularization**: Fully decoupled hardware-specific dependencies (`probe-rs`, `hidapi`) behind a `hardware` feature across all workspace crates. Fixed CI cross-compilation by correctly configuring `aether-ui` as a static library for iOS and migrating `syntect` to a pure-Rust regex engine (`fancy-regex`) to prevent macOS SDK compilation conflicts.


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
