---
description: Run Hardware-in-the-Loop (HIL) regression tests
---

This workflow executes the smoke test suite on connected hardware.

1. Ensure a debug probe (J-Link or ST-Link) and an STM32L476RG target are connected.
2. Build the latest binaries:
```bash
cargo build -p aether-core --release && cargo build -p aether-agent-api --release
```
3. Run the HIL smoke test:
// turbo
```bash
./tests/hil_smoke.sh
```
4. Verify the output ends with "HIL Smoke Test PASSED!".
