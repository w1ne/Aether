# HIL Report: Stepping & Flashing Fixes

**Date:** 2026-02-18
**Hardware:** J-Link Base + STM32L476RG
**Software:** aether-core (probe-rs 0.31)

## Flashing Verification

| Target | File Format | Status | Notes |
| :--- | :--- | :--- | :--- |
| STM32L476RG | ELF | **SUCCESS** | Enabled `keep_unwritten_bytes` (formerly `restore_unwritten_bytes`). |

**Observation:** ELF segments that were not page-aligned now flash correctly without alignment/overlap errors.

## Stepping Verification

| Command | Level | Status | Notes |
| :--- | :--- | :--- | :--- |
| Step Into | Source (IntoStatement) | **SUCCESS** | Halted at inner function correctly. |
| Step Over | Source (OverStatement) | **SUCCESS** | Advance to next line correctly. |
| Step Out | Source (OutOfStatement) | **SUCCESS** | Returned to caller correctly. |

**Observation:** High latency reported previously was not observed during this verification run. The use of `OutOfStatement` correctly implemented the "Step Out" functionality which was previously a hardware single-step.

## Build Status

- [x] `aether-core` compiles (release)
- [x] `aether-agent-api` compiles (release)
- [x] `aether-demo-fw` compiles (target thumbv7em)
