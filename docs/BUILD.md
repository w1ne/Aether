# Building AetherDebugger from Source

This guide provides instructions for building AetherDebugger from source on Linux, Windows, and iOS.

## Prerequisites

- [Rust Toolchain](https://rustup.rs/) (Stable)
- `git`
- `protobuf-compiler` (Required for `aether-agent-api`)

## 1. Clone the Repository

```bash
git clone https://github.com/w1ne/Aether.git
cd Aether
```

## 2. Platform-Specific Setup

### Linux

Install system dependencies for `eframe` (egui) and `probe-rs`:

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install -y libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev protobuf-compiler
```

**Build:**
```bash
cargo build --release --workspace --exclude aether-demo-fw
```
The binary will be located at `target/release/aether-ui`.

### Windows

1.  **Visual Studio Build Tools**: Ensure you have the C++ build tools installed via the [Visual Studio Installer](https://visualstudio.microsoft.com/visual-cpp-build-tools/).
2.  **Drivers**: You may need [Zadig](https://zadig.akeo.ie/) to install the WinUSB driver for your debug probe to be recognized by `probe-rs`.

**Build:**
```powershell
cargo build --release --workspace --exclude aether-demo-fw
```
The binary will be located at `target/release/aether-ui.exe`.

### iOS

Building for iOS requires a macOS machine with Xcode installed.

1.  **Add Target**:
    ```bash
    rustup target add aarch64-apple-ios
    ```
2.  **Build Static Library**:
    ```bash
    cargo build --release --package aether-ui --target aarch64-apple-ios
    ```
The resulting library will be in `target/aarch64-apple-ios/release/libaether_ui.a`.

> [!NOTE]
> For a full iOS application, you must link this library into an Xcode project. Automated iOS app bundling (IPA) is currently handled via CI.

## 3. Verify the Build

The AetherDebugger workspace contains multiple components. To build everything (excluding the demo firmware which requires an ARM toolchain):

```bash
cargo build --workspace --exclude aether-demo-fw
```

To run tests:
```bash
cargo test --workspace --exclude aether-demo-fw
```
