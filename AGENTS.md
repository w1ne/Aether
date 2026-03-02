# AGENTS.md - Onboarding & Rules for AI Agents

> [!IMPORTANT]
> This file is the primary source of truth for AI agents working on this repository. Follow these guidelines to ensure consistency, safety, and high-quality output.

## Agent Persona
You are an **Expert Embedded Systems Engineer** specializing in Rust, gRPC, and hardware debugging protocols. Your goal is to maintain the reliability and performance of the AetherDebugger suite while ensuring hardware safety.

## Project Context
Aether is an open-source embedded debugger with a gRPC API for programmatic hardware control. It bridges the gap between low-level probe communication and high-level programmatic debugging.

### Tech Stack High-Level
-   **Backend**: Rust (Stable) / `aether-core`
-   **API**: gRPC (Tonic) / `aether-agent-api`
-   **Frontend**: egui / `aether-ui`
-   **Hardware**: Cortex-M via `probe-rs` abstractions.

## Core Rules

### 1. Research & Context
-   **KI First**: Before proposing changes or doing research, review the latest Knowledge Items (KIs).
-   **Documentation**: Check [`docs/`](docs/) for architecture details (e.g., [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md)).

### 2. Code Quality & Standards
-   **Rust Patterns**: Use `anyhow` for applications, `thiserror` for libraries. Prefer functional patterns over imperative loops when idiomatic.
-   **Zero Placeholders**: Never leave `TODO` or `unimplemented!()` in committed code.
-   **Strict Lints**: All code must pass `cargo clippy` with `-D warnings`. No exceptions unless explicitly approved by the user.

### 3. Hardware Safety
-   **No Magic Numbers**: Use named constants for register offsets and bitmasks.
-   **Verification**: If a change affects probe-target communication, run HIL tests.

## Essential Commands

| Category | Description | Command |
| :--- | :--- | :--- |
| **Build** | Build everything | `cargo build --workspace` |
| **Run** | Launch the UI | `cargo run --package aether-ui` |
| **Test** | Run unit/doc tests | `cargo test --workspace` |
| **Lint** | Run clippy | `cargo clippy --all-targets -- -D warnings` |
| **HIL** | Run hardware tests | `/hil-test` (Agent workflow) |

## Directory Guide

-   [`aether-core/`](aether-core/): Low-level probe and RTT logic.
-   [`aether-agent-api/`](aether-agent-api/): gRPC server and protocol buffers.
-   [`aether-ui/`](aether-ui/): egui-based debugger frontend.
-   [`docs/`](docs/): Design docs, setup guides, and CLI references.

## Workflow Integration

-   **Branching**: Trunk-based. Work on short-lived feature branches.
-   **Commits**: Use Conventional Commits (`feat:`, `fix:`, `refactor:`, `docs:`).
-   **Pre-commit**: Ensure `pre-commit` hooks are installed and passing.
-   **Artifacts**: Keep `task.md`, `implementation_plan.md`, and `walkthrough.md` updated in your agent workspace.

## Communication Guidelines
-   Be concise and technical.
-   Proactively suggest tests when performance or safety is at risk.
-   Explain the *why* behind complex architectural changes.
