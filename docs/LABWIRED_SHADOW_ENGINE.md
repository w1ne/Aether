# Aether + LabWired: The Shadow Parity Engine

This document details the unique architectural integration between Aether and LabWired, specifically the "Shadow Parity" feature that distinguishes Aether from standard IDE extensions.

## Why not just use the LabWired VS Code Extension?

The LabWired extension for VS Code is a **DAP (Debug Adapter Protocol)** bridge. While powerful for isolated simulation, it suffers from "Architectural Isolation":

1.  **Session Silos**: VS Code treats every debug session as an island. You can run LabWired and a physical probe simultaneously, but they cannot "see" each other's state to perform real-time diffing.
2.  **Step Desync**: There is no way to atomically "Step" two independent VS Code sessions. Network latency and OS scheduling mean the simulator and hardware will always be out of sync.
3.  **UI Fragmentation**: Diffs must be manually compared by the human eye between two different register windows.

## The Aether "Orchestrator" Model

Aether acts as the **Unified Host** for both LabWired (Simulation) and Probe-rs (Hardware). 

### 1. Atomic Lockstep Control
Aether's core engine holds a handle to both sessions. When you click "Step," Aether:
1.  Sends a `Step` command to the physical Probe.
2.  Sends a `Step` command to LabWired.
3.  Waits for BOTH to return a `TargetHalted` event.
4.  Only THEN updates the UI.

### 2. Live Parity Diffing
After every step, Aether's **Shadow Engine** automatically performs a state comparison:
- **Registers**: Diffs R0-R15, PC, SP, and Status registers.
- **Memory**: Diffs watched variables and stack regions.
- **Peripherals**: Compares peripheral register writes recorded in the simulation vs. read from the hardware.

### 3. Agent-First Analysis
Because Aether exposes a gRPC API for the *entire orchestrated session*, an AI agent can monitor the **Divergence Stream**. If the Hardware and Simulator disagree on a branch, the agent can pause the system and analyze the DWARF info to explain the discrepancy.

## Use Case: Hardware-in-the-Loop (HIL) Hardening

Aether is used to verify that a simulator's model of a custom peripheral (e.g., a proprietary FPGA bridge) is bit-accurate to the real silicon. 

1.  Attach Aether to the Real Hardware.
2.  Attach Aether to the LabWired Model.
3.  Enable `ShadowMode`.
4.  Run a regression suite. 
5.  Aether automatically generates a **Parity Audit Log** highlighting every instruction where the physical hardware behavior diverged from the simulation code.
