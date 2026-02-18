# Aether Debugger - Use Case Scenarios (Second Pass)

This document provides a comprehensive, narrative-style description of the core use cases for the Aether Debugger. It explains the "Why" and "How" for every major functionality, including edge cases, technical interactions, and professional workflows.

---

## 1. Core Debug Control (Halt, Resume, Step)
### Background
The foundation of any debugger is the ability to control execution. Developers often need to pause the system to catch it in a specific state or "walk" through code to find logic errors.

### The Scenario
Alice is debugging a sensor initialization routine.
1. **Halt**: Alice clicks "Halt". Aether sends a `Halt` request. The UI shows a subtle "Halting..." animation before snapping to the current PC.
2. **Step**: Alice "Steps Over" a delay function. Aether calculates the return address and sets a temporary breakpoint to skip the internal delay loop.
3. **Resume**: Alice clicks "Resume". The UI removes the halt highlight and transitions back to a "Running" state.

### Edge Cases & Resiliency
- **Already Halted**: If Alice clicks "Halt" while the core is already stopped, Aether detects the state and simply refreshes the UI context without sending redundant commands.
- **Stepping in Exceptions**: If Alice steps while in a fault handler, Aether provides a warning or highlights the exception state in the status bar.

### Technical Deep Dive
- **Command**: `DebugCommand::Halt` -> `probe_rs::Core::halt()`
- **Event**: `DebugEvent::Halted { pc }` -> UI focuses on `SourceView`.

---

## 2. Fast Flash Programming
### Background
Iterative development requires fast turnaround cycles. Traditional flashing tools can be slow and brittle.

### The Scenario
Bob just optimized his motor control algorithm.
- **Action**: Bob drags `motor_ctrl.elf` into Aether. 
- **Experience**: The UI shows a "Premium" progress bar. Aether automatically selects the correct algorithm from the CMSIS-Pack.
- **Verification**: After flashing, Aether performs a checksum verify to ensure data integrity.

### Edge Cases & Resiliency
- **Flash Lock**: If the device is read-protected, Aether prompts Bob to "Unlock & Erase" rather than failing silently.
- **Power Loss**: If the cable is bumped during flashing, Aether reports a "Probe Connection Lost" error and allows Bob to retry once reconnected.

### Technical Deep Dive
- **Manager**: `FlashManager` uses `probe_rs::flashing` with multi-sector buffering for speed.
- **Events**: `FlashStatus`, `FlashProgress(0.0-1.0)`, `FlashDone`.

---

## 3. Dynamic Memory Inspection
### Background
Silent memory corruption can cause "impossible" bugs. Being able to see raw RAM state in real-time is an essential forensic tool.

### The Scenario
Charlie is investigating a stack overflow.
- **Action**: Charlie scrolls the Memory View to `0x20000000`. 
- **Experience**: Aether uses "Lazy Loading" to fetch only the visible range.
- **Watch**: Charlie marks a region as "Watch". If the core is running and the OS supports it, Aether can highlight changed bytes on the next poll.

### Edge Cases & Resiliency
- **Invalid Address**: If Charlie enters `0xDEADBEEF`, Aether validates it against the target's memory map (defined in SVD/TargetInfo) and shows a "Protected/Invalid Region" tooltip.
- **Unaligned Access**: Aether handles 8/16/32-bit alignment transparently or provides warnings for architecture-specific restrictions.

---

## 4. Register-Level Forensics
### Background
Hard-to-track bugs often involve corruption of core registers or execution context.

### The Scenario
Eve is debugging a HardFault.
- **Action**: Eve views the Registers Pane.
- **Visuals**: Changed registers are highlighted in a neon-cyan diff color.
- **Floating Point**: If she moves to the FPU tab, she sees the S0-S31 registers decoded as both Hex and Float.

### Edge Cases & Resiliency
- **Read-Only Registers**: Aether prevents editing registers that are architecturally read-only (like certain status bits).
- **Architecture Mismatch**: If Eve tries to read CSRs on a Cortex-M, Aether gracefully ignores them.

---

## 5. Precise Hardware Breakpoints
### Background
Finding where a bug occurs is often harder than fixing it. Breakpoints allow "waiting" for the bug to happen.

### The Scenario
Frank needs to know when his watchdog timer is triggered.
- **Action**: Frank clicks line 150 in `main.c`. 
- **Logic**: Aether checks the limit of the MCU's Breakpoint Unit (e.g., 6 units).
- **Warning**: If Frank sets a 7th breakpoint, Aether shows a "Hardware Limit Reached" notification.

### Edge Cases & Resiliency
- **Hitting while Halted**: Aether prevents setting breakpoints on addresses that aren't mapped to valid flash/RAM.
- **Persistence**: Breakpoints are remembered across "Reset" but cleared on "New Session" to prevent ghost halts.

---

## 6. SVD-Powered Peripheral Integration
### Background
Modern MCUs have thousands of registers. Looking up addresses in a manual is exhausting.

### The Scenario
Grace is configuring `TIM3`.
- **Action**: She selects `TIM3` from the tree.
- **Hierarchy**: She navigates `TIM3` -> `Prescaler (PSC)`.
- **Interpretation**: Aether decodes the value `1000` as "Timer Clock / 1001".

### Edge Cases & Resiliency
- **SVD Missing**: If the user hasn't loaded an SVD, Aether offers a "Search SVD" button or a generic "Base Address" entry.
- **Write-Sensitive Registers**: Aether warns Grace if she tries to write to a "Read-Only" or "Write-Once" bitfield based on SVD metadata.

---

## 7. High-Performance RTT Logging
### Background
Standard UART logging is slow and intrusive.

### The Scenario
Heidi needs telemetry from a flying drone.
- **Auto-Attach**: Aether searches for the `_SEGGER_RTT` string in the target's RAM.
- **Terminal**: Heidi sees color-coded logs (ANSI support). She can "Clear" or "Save to File" the log stream.

### Edge Cases & Resiliency
- **RTT Not Initialized**: If Aether finds the symbol but the magic sequence is invalid, it shows "RTT Pending... (waiting for target init)".
- **Buffer Overflow**: If the target sends data faster than Aether can poll, Aether reports "RTT Drop Detected" in the status bar.

---

## 8. Source-Level Debugging (DWARF)
### Background
Developers want to work in the language they wrote.

### The Scenario
Ivan is debugging a Rust application.
- **Syntax**: Aether's `SourceView` uses `syntect` for theme-aware highlighting.
- **Mapping**: When Ivan hovers over a function, Aether can (if indexed) show the address of that symbol.

### Edge Cases & Resiliency
- **Source Not Found**: If the ELF points to `/build/main.rs` but the file isn't on Ivan's disk, Aether prompts for a "Source Map" or shows the Disassembly view instead.
- **Optimized Out**: For `inline` functions, Aether shows "(Inlined)" in the breadcrumbs.

---

## 9. Call Stack Reconstruction
### Background
Knowing *how you got there* is better than just knowing where you are.

### The Scenario
Judy is inside a generic error handler.
- **Unwinding**: Aether parses the `.debug_frame` (CFI) to find where each function's return address and stack pointer were stored.
- **Visibility**: Judy sees not just function names, but the values of arguments passed to each frame.

### Edge Cases & Resiliency
- **Corrupt Stack**: If the SP is pointing to invalid RAM, Aether stops the walk and shows "Stack Corrupted / Root Frame Reached".
- **Tail-Call Optimization**: Aether correctly identifies frames even when the compiler has reused the return slot for tail calls.

---

## 10. RTOS Task Awareness (FreeRTOS)
### Background
Multi-threaded systems require seeing the state of every task.

### The Scenario
Kevin suspects a deadlock.
- **Visualization**: Kevin sees a "Task Table" with CPU usage % and Stack High-Water marks.
- **Focus**: He clicks "Focus Task", and Aether switches the Register and Stack views to show context for *that* task's saved state.

### Edge Cases & Resiliency
- **Kernel Version Mismatch**: Aether detects if the FreeRTOS version on target differs from its parser and provides a compatibility warning.
- **Veneer Frames**: Aether handles tasks that are in its "Wait" or "Tick" handlers specially.

---

## 11. Live Variable Plotting (Oscilloscope View)
### Background
Static snapshots don't tell the whole story.

### The Scenario
Leo is tuning a motor PID.
- **Interaction**: Leo clicks a variable in the Source View and selects "Add to Plot".
- **Real-Time**: He sees the position sine wave updated at 20Hz.

### Edge Cases & Resiliency
- **Variable Out of Scope**: If a plotted variable is local to a function that exited, Aether stops updating its trace and shows "(Stale)".
- **Bandwidth Limit**: Aether throttles polling if it detects that constant reading is "slowing down" the debug interface too much.

---

## 12. Remote Automation via gRPC Agent API
### Background
CI/CD pipelines and AI agents need programmatic access to hardware validation.

### The Scenario
An AI agent or script verifies boot and performs automated diagnostics.
- **Concurrency**: Aether handles a UI user and a gRPC script simultaneously, broadcasting events to both.
- **OpenClaw Integration**: An OpenClaw instance monitors the `RttEvent` stream. If it detects a "Panic" or "HardFault" string, it automatically triggers a `Halt`, captures the call stack via `GetStack`, and suggests a fix to the developer.

### Edge Cases & Resiliency
- **Control Conflict**: If the Script halts and the User resumes, Aether prioritizes the manual User intervention and notifies the gRPC clients.
- **Timeout**: gRPC calls have strict timeouts to ensure a script doesn't hang if the probe disconnects.

---

## 13. Instruction & Event Trace (ITM/SWO)
### Background
Timing issues require retrospective traces.

### The Scenario
Mia is investigating a race condition.
- **Configuration**: She sets Port 31 for "Debug Prints" and Port 0 for "Instruction Samples".
- **Decoding**: Aether decodes the raw SWO stream into human-readable events.

### Edge Cases & Resiliency
- **Baud Rate Mismatch**: Aether attempts to auto-calculate the SWO speed based on the core's reported frequency.
- **Missing SWO Pin**: If the probe or target doesn't support SWO, Aether disables the trace options.
