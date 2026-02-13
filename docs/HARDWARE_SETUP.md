# Hardware Setup

Platform-specific instructions for debug probe configuration.

## Linux

### udev Rules

Debug probes require USB permissions. Install udev rules for your probe type:

#### ST-Link (STM32 Discovery/Nucleo boards)
```bash
sudo tee /etc/udev/rules.d/99-stlink.rules << EOF
# ST-Link V2
SUBSYSTEM=="usb", ATTR{idVendor}=="0483", ATTR{idProduct}=="3748", MODE="0666"
# ST-Link V2-1
SUBSYSTEM=="usb", ATTR{idVendor}=="0483", ATTR{idProduct}=="374b", MODE="0666"
# ST-Link V3
SUBSYSTEM=="usb", ATTR{idVendor}=="0483", ATTR{idProduct}=="374e", MODE="0666"
EOF
```

#### J-Link
```bash
sudo tee /etc/udev/rules.d/99-jlink.rules << EOF
SUBSYSTEM=="usb", ATTR{idVendor}=="1366", MODE="0666"
EOF
```

#### CMSIS-DAP (DAPLink, LPC-Link2)
```bash
sudo tee /etc/udev/rules.d/99-cmsis-dap.rules << EOF
SUBSYSTEM=="usb", ATTR{idVendor}=="0d28", MODE="0666"
EOF
```

#### Apply Rules
```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Unplug and replug your probe.

### Dependencies

```bash
# Ubuntu/Debian
sudo apt install libusb-1.0-0-dev libudev-dev

# Fedora
sudo dnf install libusb-devel systemd-devel

# Arch
sudo pacman -S libusb systemd
```

## Windows

### Driver Installation

Windows requires WinUSB driver for most debug probes.

#### Method 1: Zadig (Recommended)
1. Download [Zadig](https://zadig.akeo.ie/)
2. Connect your debug probe
3. Run Zadig
4. Select your probe from the dropdown
5. Select "WinUSB" as the driver
6. Click "Replace Driver"

#### Method 2: Official Drivers
- **ST-Link**: Install [STM32 ST-LINK Utility](https://www.st.com/en/development-tools/stsw-link004.html)
- **J-Link**: Install [J-Link Software Pack](https://www.segger.com/downloads/jlink/)

### Build Dependencies

Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) for Rust compilation.

## macOS

### Permissions

macOS requires no special setup for most probes. USB access is granted automatically.

### Dependencies

```bash
# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install libusb
brew install libusb
```

## Wiring

### SWD (Serial Wire Debug)

Minimum 3-wire connection:

| Probe Pin | Target Pin | Description |
|-----------|------------|-------------|
| SWDIO     | SWDIO      | Data line   |
| SWCLK     | SWCLK      | Clock line  |
| GND       | GND        | Ground      |

Optional:
- **VCC**: Power target from probe (check voltage compatibility!)
- **NRST**: Reset line for hard resets
- **SWO**: Trace output (for ITM/SWO logging)

### JTAG

5-wire connection (less common for Cortex-M):

| Probe Pin | Target Pin |
|-----------|------------|
| TDI       | TDI        |
| TDO       | TDO        |
| TCK       | TCK        |
| TMS       | TMS        |
| GND       | GND        |

## Verification

Test your setup:

```bash
# List detected probes
probe-rs list

# Should show your probe, e.g.:
# [0]: ST-Link V2 (VID: 0483, PID: 3748, Serial: 066DFF575251897267072518)
```

If no probes appear:
- **Linux**: Check udev rules
- **Windows**: Verify WinUSB driver installed
- **All**: Try different USB cable/port

## Troubleshooting

**"Permission denied" (Linux)**
- Verify udev rules installed correctly
- Check user is in `plugdev` group: `sudo usermod -aG plugdev $USER`
- Log out and back in

**"Device not found" (Windows)**
- Use Zadig to install WinUSB driver
- Disable "Driver Signature Enforcement" if needed

**"Connection failed"**
- Check SWD wiring (SWDIO, SWCLK, GND)
- Ensure target is powered
- Try lower SWD speed (Aether auto-negotiates)

**Multiple probes detected**
- Aether will list all probes
- Select the correct one by serial number
