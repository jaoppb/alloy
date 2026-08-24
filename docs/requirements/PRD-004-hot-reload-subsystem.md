# PRD-004: Runtime Hot-Reload Subsystem

- **Status**: Accepted
- **Author**: Core Architecture Team
- **Date**: 2026-08-22
- **Target Release**: v0.1.0-alpha

---

## 1. Executive Summary

Alloy provides a real-time developer and user experience where scripts running any subsystem (HTML parsing, CSS cascade,
network routing, UI widgets) can be modified and reloaded instantly during browser execution without restarting the
process or losing underlying DOM/network session state.

---

## 2. Problem Statement

Rebuilding and restarting native browser applications introduces significant latency into the development and
customization loop. If hot-reloading is implemented naively by blindly re-running script files into active memory, it
can lead to state corruption, memory leaks, or crashes if the new script has syntax errors.

---

## 3. Hot-Reload Architecture & Protocol

### 3.1 Dual Trigger Mechanism

Hot-reloads are triggered through two channels:

1. **Filesystem Watcher**: Background thread monitoring user script directories (`~/.alloy/scripts/` or project
   `scripts/`) using asynchronous file change notifications.
2. **DevTools / IPC Command**: Explicit RPC command sent from the DevTools panel or CLI command.

### 3.2 Atomic Compilation & Swap Flow

```text
[ File Modified on Disk ]
          │
          ▼
[ File Watcher Debounce (50ms) ]
          │
          ▼
[ Background Engine AST Validation & Compilation ]
          │
    ┌─────┴─────────────────────────┐
    │                               │
[ Compilation Success ]     [ Compilation Error ]
    │                               │
    ▼                               ▼
[ Atomic Pointer Swap (Arc<AST>) ]  [ Keep Previous Active AST ]
    │                               │
    ▼                               ▼
[ Reset Script Engine Scope ]       [ Emit Error Event to DevTools ]
    │
    ▼
[ Re-bind Rust Domain State ]
    │
    ▼
[ Invoke `on_reload()` Hook ]
```

---

## 4. State Management Rules

1. **Rust Holds Ground Truth (Skeleton)**: All persistent state (DOM tree nodes, open sockets, cache entries, layout
   boxes) resides in Rust memory and is unaffected by script reloading.
2. **Stateless Script Scopes**: Script scopes are discarded and cleanly re-initialized upon reload to prevent variable
   leaks or state desynchronization.
3. **Rollback on Error**: If a modified script has compilation or syntax errors, the subsystem continues executing the
   previous valid AST, and logs the syntax error with line/column pointers to DevTools.

---

## 5. Acceptance Criteria

- [ ] File watcher detects `.rhai` script modifications with debouncing.
- [ ] Successful script edits compile in background and swap atomically.
- [ ] Script with syntax errors does not replace the running AST and logs diagnostics.
- [ ] Active DOM and window state remain intact after multiple hot-reloads.
