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

## 3. End-to-End (E2E) Tests (Behavioral)
- **Location**: `aether-core/tests/e2e_scenarios.rs`
- **Scope**: Validating high-level user workflows as documented in [USE_CASES.md](file:///home/andrii/Projects/AetherDebugger/docs/USE_CASES.md).
- **Command**: `cargo test -p aether-core --test e2e_scenarios`

## 4. Benchmarking
- **Location**: `benchmarks/`
- **Scope**: Performance tracking for critical paths (e.g., DAP message processing).
- **Command**: `cargo bench`

## Pre-commit Hooks
Before every commit, the following are checked:
- Rust formatting (`rustfmt`)
- Linting (`clippy`)
- Compilation (`cargo check`)
