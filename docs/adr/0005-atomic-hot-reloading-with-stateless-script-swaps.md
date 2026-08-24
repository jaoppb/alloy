# ADR-0005: Atomic Hot-Reloading with Stateless Script Swaps

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-08-22

---

## Context and Problem Statement

To provide an instant development feedback loop and allow live user reconfiguration, Alloy must support hot-reloading
scripts while the browser is running. How should we handle script compilation errors, state preservation, and dynamic
swapping without causing memory corruption or unexpected side effects?

---

## Decision Drivers

- Script edits must take effect immediately without restarting the browser.
- Broken edits (syntax errors, invalid types) must not crash the running subsystem.
- State in the Rust core must remain persistent across reloads.
- No memory leaks or orphaned closures from old scripts.

---

## Considered Options

- **Option 1**: Asynchronous background AST validation with atomic pointer swap (`Arc<CompiledAST>`), stateless script
  scope reset, and rollback on compilation error.
- **Option 2**: Dynamic in-place variable mutation and hot state serialization/deserialization.
- **Option 3**: Cold restart of the browser process on script changes.

---

## Decision Outcome

Chosen option: **Option 1**.

### Implementation Mechanics

1. A filesystem watcher (`notify` crate) monitors script files in background threads.
2. When a file modification event occurs, the engine compiles the new AST on a worker thread.
3. If compilation fails:
    - The error is dispatched to the DevTools event stream.
    - The subsystem continues executing using the existing, valid AST.
4. If compilation succeeds:
    - The compiled AST is atomically swapped via `Arc<CompiledAST>`.
    - The subsystem's `ExecutionContext` scope is cleared and re-bound with native Rust domain handles.
    - The subsystem's `on_reload()` lifecycle hook is invoked.

### Consequences

- **Positive**:
    - 100% atomic: Subsystems never enter a half-compiled, broken state.
    - Rust retains domain state (tabs, DOM trees, network requests) seamlessly.
    - Zero memory leaks from old script scopes.
- **Negative**:
    - Script-local temporary variables are reset upon reload (which is the intended behavior under the Skeleton & Muscle
      model).
