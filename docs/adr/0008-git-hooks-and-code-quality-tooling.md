# ADR-0008: Git Hooks and Code Quality Tooling

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-08-22

---

## Context and Problem Statement

To maintain code and documentation quality across the modular Rust codebase, we need automated formatting, linting,
testing, and build validation before changes are committed or pushed to remote repositories.

---

## Decision Drivers

- Enforce uniform code and Markdown formatting automatically without manual developer intervention.
- Detect Rust compiler warnings and lint violations early before CI runs.
- Prevent broken builds or failing tests from being pushed.
- Avoid git lock race conditions during staged file formatting.

---

## Considered Options

- **Option 1**: **Lefthook** managing sequential pre-commit (`cargo fmt`, `clippy`, `prettier`, `markdownlint-cli2` with
  `stage_fixed: true`) and pre-push (`cargo test`, `cargo check`).
- **Option 2**: Python-based `pre-commit` framework.
- **Option 3**: Husky + lint-staged in Node.js ecosystem.
- **Option 4**: Unenforced developer local checks, relying entirely on CI.

---

## Decision Outcome

Chosen option: **Option 1 (Lefthook + pnpm + Cargo)**.

### Configuration Details

1. **Pre-Commit (Sequential to prevent git lock contention)**:
    - **Rust Formatting**: `cargo fmt` with automatic staging of fixes (`stage_fixed: true`).
    - **Rust Linting**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
    - **Markdown Formatting**: `prettier --write` (tabs, tab-width 4) with automatic staging (`stage_fixed: true`).
    - **Markdown Linting**: `markdownlint-cli2` enforcing strict Markdown standards.
2. **Pre-Push (Sequential)**:
    - **Test Suite**: `cargo test --workspace`.
    - **Compilation Check**: `cargo check --workspace --all-targets`.

### Consequences

- **Positive**:
    - Ultra-fast native execution via Lefthook binary.
    - Automatic formatting fixes staged seamlessly without manual git add steps.
    - Ensures clean git history and high code/doc consistency.
- **Negative**:
    - Requires `pnpm` and `cargo` installed in developer environments.
