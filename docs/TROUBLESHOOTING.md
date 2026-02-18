# Troubleshooting

Common errors and solutions.

## Probe Connection Issues

### "No probes found"

**Symptoms**: Aether shows empty probe list.

**Solutions**:
1. **Check USB connection**
   - Try different USB port
   - Try different USB cable (data cable, not charge-only)

2. **Linux: Install udev rules**
   ```bash
   # See docs/HARDWARE_SETUP.md for your probe type
   sudo udevadm control --reload-rules
   sudo udevadm trigger
   ```

3. **Windows: Install WinUSB driver**
   - Use [Zadig](https://zadig.akeo.ie/)
   - Select your probe → WinUSB → Replace Driver

4. **Verify with probe-rs**
   ```bash
   probe-rs list
   ```

### "Probe connected but target not detected"

**Symptoms**: Probe appears but "Connect" fails.

**Solutions**:
1. **Check SWD wiring**
   - Minimum: SWDIO, SWCLK, GND
   - Verify correct pin mapping for your target

2. **Power target board**
   - Ensure target has power (LED indicator)
   - Check voltage (3.3V typical for Cortex-M)

3. **Check SWD pins not used for GPIO**
   - Some targets disable SWD after boot
   - Hold NRST low during connection
   - May need "Connect Under Reset" option

4. **Try lower SWD speed**
   - Long wires or poor connections need slower speeds
   - Aether auto-negotiates but may fail on marginal connections

## Flash Programming Issues

### "Flash failed: Erase error"

**Symptoms**: Flashing stops during erase phase.

**Solutions**:
1. **Check flash protection**
   - Device may be read-protected
   - Use ST-Link Utility or J-Link Commander to unlock
   - **Warning**: Unlocking erases entire flash

2. **Verify power stability**
   - Insufficient power during erase can cause failures
   - Use external power supply, not probe VCC

3. **Check flash algorithm**
   - Aether auto-selects from CMSIS-Pack
   - Verify correct chip variant selected

### "Flash failed: Verify error"

**Symptoms**: Programming completes but verification fails.

**Solutions**:
1. **Check target voltage**
   - Voltage drop during programming can corrupt data
   - Measure VDD during flash operation

2. **Bad flash sectors**
   - Try erasing full chip first
   - Some sectors may be worn out (rare)

## RTT Issues

### "RTT not attaching"

**Symptoms**: "Attach RTT" button does nothing or shows "Pending...".

**Solutions**:
1. **Ensure firmware initialized RTT**
   ```rust
   // Firmware must call RTT init
   rtt_target::rtt_init_print!();
   ```

2. **Halt target before attaching**
   - Click "Halt"
   - Then "Attach RTT"
   - RTT scans RAM for control block

3. **Check RTT control block in RAM**
   - Must be in `.bss` or `.data` section
   - Not in flash or uninitialized memory

4. **Increase RTT buffer size**
   - Default 1KB may be too small for high-rate logging
   - Increase in firmware RTT configuration

### "RTT Drop Detected"

**Symptoms**: Status bar shows "RTT Drop Detected".

**Solutions**:
1. **Reduce logging rate**
   - Target is writing faster than Aether can read
   - Add delays or reduce log verbosity

2. **Increase polling rate**
   - Aether polls at 100Hz by default
   - Higher rates reduce drops but increase CPU usage

3. **Increase RTT buffer size**
   - Larger buffers absorb bursts better

## Performance Issues

### "Slow step latency (>100ms)"

**Symptoms**: Stepping feels sluggish.

**Solutions**:
1. **Check USB connection**
   - USB 2.0 hub can add latency
   - Connect probe directly to computer

2. **Disable unnecessary views**
   - Memory/Register auto-refresh adds overhead
   - Close unused tabs

3. **Reduce symbol file size**
   - Strip debug symbols: `strip --strip-debug firmware.elf`
   - Or use release build with minimal debug info

### "UI freezing during operations"

**Symptoms**: UI becomes unresponsive.

**Solutions**:
1. **Update to latest Aether**
   - Performance improvements in newer versions

2. **Check system resources**
   - Aether needs ~100MB RAM
   - CPU usage should be <10% when idle

3. **Report bug**
   - UI should never freeze
   - File issue with reproduction steps

## Build Issues

### "cargo build fails: libusb not found"

**Symptoms**: Compilation error about missing libusb.

**Solutions**:
```bash
# Linux
sudo apt install libusb-1.0-0-dev libudev-dev

# macOS
brew install libusb

# Windows
# Install Visual Studio Build Tools
```

### "cargo build fails: linker errors"

**Symptoms**: Linking fails with undefined references.

**Solutions**:
1. **Clean build**
   ```bash
   cargo clean
   cargo build
   ```

2. **Update Rust**
   ```bash
   rustup update
   ```

## Still Stuck?

1. **Check GitHub Issues**: [github.com/w1ne/Aether/issues](https://github.com/w1ne/Aether/issues)
2. **Enable debug logging**:
   ```bash
   RUST_LOG=debug ./aether-ui
   ```
3. **File a bug report** with:
   - Aether version (`git rev-parse HEAD`)
   - Probe type and firmware version
   - Target chip
   - Full error message
   - Steps to reproduce
