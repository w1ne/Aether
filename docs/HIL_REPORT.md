# HIL Test Report: STM32L476RG

**Date**: 2026-02-18
**Device Under Test**: STM32L476RG Nucleo-64
**Debugger**: ST-Link V2-1 (On-board)

## Executive Summary
Hardware-in-the-Loop (HIL) tests were successfully conducted to verify the Aether Debugger's performance on real silicon. The debugger successfully demonstrated core control, memory access, and event streaming.

## Connection Requirements
Critical findings for successful hardware connection:

- **Chip Name**: Use the specific variant name `STM32L476RGTx` instead of the generic family name.
- **Protocol**: Explicitly select **SWD** (`--protocol swd`). The ST-Link V2-1 on this board defaults to JTAG, which fails to detect the target.
- **Reset Strategy**: Standard SWD attachment works reliably. `attach_under_reset` is not strictly necessary and may encounter timeouts if the target reset pin configuration is non-standard.

## Verified Features

| Feature | Results | Notes |
| :--- | :--- | :--- |
| **Connection (SWD)** | ✅ Success | Reliable attachment using `STM32L476RGTx`. |
| **Halt / Resume** | ✅ Success | Rapid response to control commands. |
| **Register Access** | ✅ Success | Verified R0-R15 read/write correctness. |
| **Memory Access** | ✅ Success | Verified 0x08000000 (Flash) reads. |
| **Event Stream** | ✅ Success | Halted events correctly propagated to Agent API. |
| **Basic Stepping** | ✅ Success | Single `core step` instruction verified. |

## Recommendations for Users

1.  **Don't Guess Chip Names**: Use a target listing utility if unsure of the exact suffix (e.g., `RGTx`).
2.  **Force SWD**: For all STM32 Nucleo/Discovery boards, pass `--protocol swd` to prevent protocol mismatch errors.
3.  **Check Voltages**: Ensure the board is powered via USB or external supply; the ST-Link reports target voltage which defaults to ~3.3V.

## Known Limitations / Future Work
- **Step Over/Into**: Encountered high latency/hangs on hardware; likely due to temporary breakpoint management logic.
- **Flashing**: ELF parsing issues observed during high-level CLI flash commands. Use specialized flashing tools or investigate ELF segment alignment.
