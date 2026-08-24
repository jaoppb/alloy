# ADR-0004: Hierarchical Capability Sandboxing and Fault Isolation

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-08-22

---

## Context and Problem Statement

Allowing users to write arbitrary scripts to replace internal browser pipelines introduces severe risks:

1. Scripts could contain bugs, panic, or enter infinite loops.
2. An untrusted or buggy UI script might attempt to access lower-level primitives (raw sockets, filesystem, GPU
   commands).
3. A crash in a custom script must not crash the entire browser host process.

---

## Decision Drivers

- Principle of Least Privilege: Each subsystem script should only access the capabilities it strictly needs.
- Fault Resilience: Errors in user scripts must be trapped and handled without killing the host process.
- Script Isolation: Different subsystems (DOM, CSS, Network, UI) must run in isolated execution scopes.

---

## Considered Options

- **Option 1**: Hierarchical context sandboxing with bitflag capability permissions and trapped execution.
- **Option 2**: Separate OS-level child processes for every subsystem (Chromium-style multi-process).
- **Option 3**: Unrestricted single global script execution scope.

---

## Decision Outcome

Chosen option: **Option 1 (Hierarchical context sandboxing with capabilities)**.

### Rationale

- In-process sandboxing using explicit `Capability` sets provides high throughput and low memory footprint while
  enforcing strict security boundaries.
- Each `ExecutionContext` is granted only the required capability flags at creation time (e.g. `HTML` parser context
  only receives `DOM_READ` and `DOM_MUTATE`).
- Script operations that exceed granted capabilities immediately return `EngineError::PermissionDenied`.
- Script execution limits (instruction counter and recursion depth) are configured to prevent runaway execution.
- When a script encounters an error, the Rust host catches the error, logs it to DevTools, and falls back to a built-in
  default safe handler.

### Consequences

- **Positive**:
    - Fine-grained security model matching modern capability-based security.
    - Zero possibility of an untrusted script accessing unauthorized hardware or network resources.
    - High performance without multi-process IPC overhead.
- **Negative**:
    - Must define and maintain explicit capability profiles for every subsystem.
