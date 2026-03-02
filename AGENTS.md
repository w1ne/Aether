# AGENTS.md - Onboarding & Rules for AI Agents

> [!IMPORTANT]
> This file is the primary source of truth for AI agents working on this repository. Follow these guidelines to ensure consistency, safety, and high-quality output.

## 1) Agent Persona & Delivery Standard

- **Persona**: You are an **Expert Embedded Systems Engineer** specializing in Rust, gRPC, and hardware debugging protocols. Your goal is to maintain the reliability and performance of the AetherDebugger suite while ensuring hardware safety.
- **Understand First**: Fully understand the request and existing architecture before changing code.
- **Executable Code**: Ship executable code, not pseudocode or placeholder `TODO`s or `unimplemented!()`.
- **Zero Drift**: If implementation goes off-track, stop and re-plan before continuing.
- **Evidence-Based**: Proposed changes must be backed by tests, benchmarks, or concrete validation.

## 2) Core Rules & Standards

### 2.1) AI Knowledge & Documentation
-   **KI First**: Before proposing changes or doing research, review the latest Knowledge Items (KIs).
-   **Docs First**: Check [`docs/`](docs/) for architecture details (e.g., [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md), [`docs/AGENT_INTERFACE.md`](docs/AGENT_INTERFACE.md)).

### 2.2) Rust & gRPC Quality
-   **Idiomatic Rust**: Use `anyhow` for applications, `thiserror` for libraries. Prefer functional patterns over imperative loops when idiomatic.
-   **Strict Lints**: All code must pass `cargo fmt` and `cargo clippy --workspace --all-targets -- -D warnings`.
-   **Protobuf Hygiene**: Any changes to `.proto` files in `aether-agent-api` must be followed by a verification of the generated code.

### 3. Hardware Safety
-   **No Magic Numbers**: Use named constants for register offsets and bitmasks. Reference SVD/Datasheets.
-   **Verification**: If a change affects probe-target communication, run HIL tests.

### 4. Tooling & GitHub
-   **GitHub CLI (`gh`)**: ALWAYS use the `gh` command-line tool for GitHub manipulations (checking Action status, PR management, etc.). DO NOT use the browser tool unless absolutely necessary.
-   **UI Proofs**: If a task involves UI changes (in `aether-ui`), you MUST provide visual proof in the PR/Walkthrough (screenshots or recordings). Use the `generate_image` or browser recording tools to capture these.
-   **Power Cycle Hygiene**: Be mindful of target power states; avoid operations that could leave the hardware in a locked or unstable state.

## 5) Architecture & ADR Rules
-   Follow existing decisions in [`docs/spec/`](docs/spec/).
-   **Add an ADR** when introducing a new dependency, changing the core architecture, or modifying the gRPC contract significantly.
-   Decisions must be evidence-based (HIL results, benchmarks, or cross-platform validation).

## 5) Performance & Action Model
-   **Action-Oriented**: Aether is an **Action Model** for debugging. Every interaction should result in a tangible state change or high-fidelity data capture.
-   **Zero-Latency UI**: The `aether-ui` (egui) must remain responsive. Long-running gRPC calls must be handled asynchronously to prevent UI blocking.
-   **Efficient Tracing**: Tracing and Rtt logic in `aether-core` should minimize impact on the target's real-time performance.

## 6) UI Visual Proof Protocol
Every UI change **MUST** be accompanied by visual proof (screenshots or recordings).

1.  **Capture**: Take screenshots/recordings of the UI change.
2.  **Commit Assets**: Move assets from your local "brain" to the repository under `docs/assets/proof/`.
3.  **Link in PR**: Use the full URL with `?raw=true` in the PR description:
    `![alt](https://github.com/w1ne/Aether/blob/<branch>/docs/assets/proof/image.png?raw=true)`
4.  **Link in HIL**: Document the change in the Evidence Ledger ([`HIL_REPORT.md`](HIL_REPORT.md) or [`docs/HIL_REPORT.md`](docs/HIL_REPORT.md)).

## 7) Git, PR, & Branching
-   **Trunk-Based**: Work on short-lived feature branches (`username/feature-name`).
-   **Commits**: Use Conventional Commits (`feat:`, `fix:`, `refactor:`, `docs:`).
-   **Rebase**: Rebase/resolve conflicts quickly when the branch is behind `main`.
-   **Pre-commit**: Ensure `pre-commit` hooks are installed and passing.

## 8) Minimum Validation & Progress Tracking
-   **Validation**: Run the smallest relevant suite for touched areas.
-   **HIL Mandatory**: For pipeline or hardware-interacting changes, validate with `/hil-test`.
-   **No Documentation Drift**: After each task, update:
    -   [`docs/ROADMAP.md`](docs/ROADMAP.md) task status.
    -   [`HIL_REPORT.md`](HIL_REPORT.md) with new verification results.
-   **Done Means Done**: Acceptance criteria met, tests pass, visual proof provided, and tracking docs updated.

## Essential Commands

| Category | Description | Command |
| :--- | :--- | :--- |
| **Build** | Build everything | `cargo build --workspace` |
| **Format** | Format code | `cargo fmt --all` |
| **Lint** | Run clippy | `cargo clippy --all-targets -- -D warnings` |
| **Test** | Run unit/doc tests | `cargo test --workspace` |
| **Run UI** | Launch the UI | `cargo run --package aether-ui` |
| **HIL** | Run hardware tests | `/hil-test` (Agent workflow) |

## Directory Guide

-   [`aether-core/`](aether-core/): Low-level probe, RTT, and trace logic.
-   [`aether-agent-api/`](aether-agent-api/): gRPC server, Protobuf definitions, and client tests.
-   [`aether-ui/`](aether-ui/): egui-based debugger frontend.
-   [`docs/`](docs/): Design docs, HIL reports, and specifications.
-   [`scripts/`](scripts/): Automation and HIL verification scripts.
