
## Reference Setup: STM32L476 Nucleo

We provide a reference firmware project `aether-demo-fw` configured for the STM32L476RG Nucleo board to validate the debugger.

### Prerequisites

- **Board**: NUCLEO-L476RG
- **Probe**: On-board ST-Link (default) or external J-Link/ST-Link.

### Building Firmware

The firmware is located in `aether-demo-fw`.

```bash
cd aether-demo-fw
cargo build --release
```

### Flashing & Running

To flash and run using `probe-rs` (configured in `.cargo/config.toml`):

```bash
cd aether-demo-fw
cargo run --release
```

### 3. Connect and Debug
Aether supports a **Zero-Config** experience. You don't need to specify chip names or protocols upfront.

1. **Launch the Daemon**:
   ```bash
   aether-daemon
   ```

2. **Discover and Attach**:
   In another terminal, use the CLI to find your board and connect:
   ```bash
   # List probes to confirm connection
   aether-cli probe list

   # Attach to target
   aether-cli probe attach --chip auto
   ```

3. **Verify**:
   ```bash
   aether-cli status
   ```

*Note: If auto-detection fails, you can explicitly attach using `aether-cli probe attach --chip STM32L476RGTx`.*
   - **Connect**: Use the daemon URL (default: `http://localhost:50051`).
   - **ELF**: Load `aether-demo-fw/target/thumbv7em-none-eabihf/release/aether-demo-fw`.
   - **Variables**: Look for `counter`.
   - **Breakpoints**: Set a breakpoint in `do_work`.

### Troubleshooting Connection

- **"JtagNoDeviceConnected"**: The probe is trying JTAG. Most STM32 Nucleos use SWD. Force SWD with `--protocol swd`.
- **"No chip found"**: Verify you are using `STM32L476RGTx`. Generic `STM32L476` may fail if the flash variant suffix is missing.
- **Timeout under reset**: High-density STM32 chips sometimes fail `attach_under_reset`. Try connecting without the flag first.

### Wiring External Probes (Optional)

If using an external J-Link or ST-Link instead of the on-board one:

1. Remove CN2 jumpers (disconnects on-board ST-Link).
2. Connect to CN4 (SWD connector) or Morpho headers:

| Signal | CN4 Pin | Morpho Pin |
|--------|---------|------------|
| SWCLK  | 2       | PA14       |
| SWDIO  | 4       | PA13       |
| GND    | 3       | GND        |
| NRST   | 5       | NRST       |
| 3.3V   | 1       | 3.3V       |

Ensure the board is powered (e.g., via USB or Vin).
