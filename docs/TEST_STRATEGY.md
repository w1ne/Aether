# Testing Strategy

AetherDebugger employs a multi-layered testing strategy to balance development speed with system reliability.

## 1. Unit Tests (Fast, Granular)
- **Location**: Inline in `src/*.rs` or `src/tests.rs` modules.
- **Scope**: Private functions, individual logic components.
- **Command**: `cargo test`

## 2. Integration Tests (Component-to-Component)
- **Location**: `tests/*.rs`
- **Scope**: Public APIs of crates, interaction between `aether-core` and its dependencies. Mocking external hardware where necessary.
- **Command**: `cargo test --test integration_tests`

## 3. End-to-End (E2E) Tests (System-wide)
- **Location**: `tests/e2e/` (planned)
- **Scope**: UI interactions triggering DAP server commands and verifying responses.
- **Command**: `npm test` (in `aether-ui`) or specialized test runners.

## 4. Benchmarking
- **Location**: `benchmarks/`
- **Scope**: Performance tracking for critical paths (e.g., DAP message processing).
- **Command**: `cargo bench`

## Pre-commit Hooks
Before every commit, the following are checked:
- Rust formatting (`rustfmt`)
- Linting (`clippy`)
- Compilation (`cargo check`)
