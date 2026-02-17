# Quick Start Guide

Get debugging in 5 minutes.

## Prerequisites

- Debug probe (ST-Link, J-Link, or CMSIS-DAP)
- ARM Cortex-M target board
- Rust toolchain installed

## 1. Install Aether

```bash
git clone https://github.com/w1ne/Aether.git
cd Aether
cargo build --release
```

Binary will be at `target/release/aether-ui`.

## 2. Connect Hardware

1. Connect debug probe to your computer via USB
2. Connect probe to target board (SWD: SWDIO, SWCLK, GND, optionally VCC)
3. Power on target board

### Linux: udev Rules
```bash
# ST-Link
echo 'SUBSYSTEM=="usb", ATTR{idVendor}=="0483", MODE="0666"' | sudo tee /etc/udev/rules.d/99-stlink.rules

# J-Link
echo 'SUBSYSTEM=="usb", ATTR{idVendor}=="1366", MODE="0666"' | sudo tee /etc/udev/rules.d/99-jlink.rules

sudo udevadm control --reload-rules
```

### Windows
May need [Zadig](https://zadig.akeo.ie/) to install WinUSB driver for your probe.

## 3. Launch Aether

```bash
./target/release/aether-ui
```

## 4. First Debug Session

### Connect to Probe
1. Aether auto-detects connected probes
2. Select your probe from the list
3. Click **"Connect"**
4. Target info appears (chip name, flash/RAM size)

### Flash Firmware
1. Click **"Flash"** button
2. Select your `.elf` or `.bin` file
3. Wait for progress bar to complete
4. Firmware is now running on target

### Debug
1. Click **"Halt"** to pause execution
2. View current state:
   - **Registers** tab: R0-R15, PC, SP
   - **Memory** tab: Enter address (e.g., `0x20000000`)
   - **Disassembly** tab: Current instruction
3. Click **"Step"** to execute one instruction
4. Click **"Resume"** to continue execution

### RTT Logging
1. Click **"RTT"** tab
2. Click **"Attach RTT"**
3. Select channel (usually "0: Terminal")
4. See live logs from your firmware

## Common Issues

**"No probes found"**
- Check USB connection
- Linux: Verify udev rules installed
- Windows: Install WinUSB driver via Zadig

**"Flash failed"**
- Ensure target is powered
- Check SWD connections (SWDIO, SWCLK, GND)
- Try "Reset" button before flashing

**"RTT not attaching"**
- Ensure firmware initialized RTT (call `rtt_init()`)
- Halt target, then click "Attach RTT"
- Check RTT control block is in RAM

## Next Steps

- [Hardware Setup](HARDWARE_SETUP.md) - Detailed probe configuration
- [Use Cases](USE_CASES.md) - Comprehensive feature guide
- [Troubleshooting](TROUBLESHOOTING.md) - Common errors and solutions
