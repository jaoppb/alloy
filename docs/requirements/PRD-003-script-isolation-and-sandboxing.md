# PRD-003: Hierarchical Capability Sandboxing & Script Isolation

- **Status**: Accepted
- **Author**: Security & Core Architecture Team
- **Date**: 2026-08-22
- **Target Release**: v0.1.0-alpha

---

## 1. Executive Summary

Alloy gives users full power to replace and customize internal scripts. Because user-written scripts may contain fatal
logic bugs, infinite loops, or malicious actions, the browser must enforce strict **Hierarchical Capability Sandboxing**
and **Fault Isolation**. A broken script in one subsystem (e.g. UI or HTML parser) must never compromise host memory,
hijack network sockets, or crash the browser host process.

---

## 2. Threat & Failure Model

1. **Buggy User Script**: A custom script contains a syntax error or logic panic during page rendering.
2. **Infinite Loop**: A script enters an unbounded loop during DOM traversal or CSS cascade computation.
3. **Privilege Escalation**: A UI-level tab script attempts to read arbitrary local disk files or bind to TCP sockets.
4. **Context Pollution**: A script in Tab A attempts to read or mutate the execution state or variables of Tab B.

---

## 3. Capability Security Architecture

### 3.1 Capability Sets (`core/engine`)

Capabilities are explicit bitflags or typed permissions passed during `ExecutionContext` creation:

```rust
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Capability: u32 {
        const DOM_READ         = 1 << 0;
        const DOM_MUTATE       = 1 << 1;
        const NETWORK_FETCH    = 1 << 2;
        const NETWORK_LISTEN   = 1 << 3;
        const FS_READ_SCRIPTS  = 1 << 4;
        const FS_WRITE_CACHE   = 1 << 5;
        const GRAPHICS_DRAW    = 1 << 6;
        const WINDOW_MANAGE    = 1 << 7;
        const DEVTOOLS_INSPECT = 1 << 8;
    }
}
```

### 3.2 Subsystem Capability Profiles

| Subsystem                      | Granted Capabilities                         | Denied Capabilities                  |
| ------------------------------ | -------------------------------------------- | ------------------------------------ |
| **DOM Parser / HTML Engine**   | `DOM_READ`, `DOM_MUTATE`                     | `NETWORK_*`, `FS_*`, `WINDOW_MANAGE` |
| **CSS Cascade / Style Engine** | `DOM_READ`, `GRAPHICS_DRAW`                  | `DOM_MUTATE`, `NETWORK_*`, `FS_*`    |
| **Network Interceptor**        | `NETWORK_FETCH`, `FS_WRITE_CACHE`            | `WINDOW_MANAGE`, `GRAPHICS_DRAW`     |
| **UI & Window Manager**        | `WINDOW_MANAGE`, `GRAPHICS_DRAW`, `DOM_READ` | `NETWORK_LISTEN`, `FS_WRITE_CACHE`   |

---

## 4. Fault Isolation & Fallback Strategies

When a user script fails:

1. **Trapped Execution**: The runtime engine catches the error via `Result<T, EngineError>`.
2. **Error Logging**: The failure is reported to the DevTools event bus with stack trace and offending script path.
3. **Default Fallback**: The host subsystem falls back to the embedded default Rust implementation (or bundled safe
   fallback script), ensuring the page continues to render.
4. **Non-Corrupting Scope Reset**: The execution context scope is flagged for clean re-initialization or hot-reload.

---

## 5. Acceptance Criteria

- [ ] Capability verification enforced at every native function binding.
- [ ] Attempting to call an unauthorized capability returns `EngineError::PermissionDenied`.
- [ ] Subsystems maintain isolated `ExecutionContext` instances with separate scopes.
- [ ] A panicking/erroring script in the DOM module does not panic the Rust host process and invokes fallback handler.
