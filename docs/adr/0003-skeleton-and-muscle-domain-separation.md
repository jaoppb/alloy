# ADR-0003: Skeleton and Muscle Domain Separation

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-08-22

---

## Context and Problem Statement

A web browser contains both rigid, high-performance data structures (DOM trees, render trees, memory buffers, network
sockets) and fluid, highly customizable behavioral workflows (tab management, user shortcuts, network interception,
layout orchestration). Where should the boundary lie between native compiled code and interpreted scripts?

---

## Decision Drivers

- Maintain maximum execution speed and memory safety for core data structures.
- Maximize user customizability and script hot-swappability.
- Avoid memory fragmentation and garbage collection overhead in core data models.

---

## Considered Options

- **Option 1**: **Skeleton & Muscle Pattern**: Rust owns all data structures, memory allocation, and OS I/O (Skeleton);
  runtime scripts define the execution flow, dispatch logic, and event responses (Muscle).
- **Option 2**: Pure Script-Driven: Build the DOM and data structures directly inside the scripting runtime.
- **Option 3**: Pure Rust with static plugin crates: Require recompiling Rust shared objects (`.so` / `.dylib`) for
  customization.

---

## Decision Outcome

Chosen option: **Option 1 (Skeleton & Muscle Pattern)**.

### Rationale

1. **Rust Core (Skeleton)**:
    - Data structures (`Node`, `Element`, `StyleSheet`, `HttpRequest`, `BitmapBuffer`) are strictly typed Rust structs.
    - Heavy computational algorithms (HTML tokenization, GPU rasterization, cryptographic TLS) remain native compiled
      code.
2. **Runtime Script (Muscle)**:
    - Scripts handle event routing, policy decisions, pipeline composition, and user overrides.
    - When an event occurs (e.g. `on_navigate`, `on_dom_mutation`), Rust invokes the script hook with domain
      handles/references.

### Consequences

- **Positive**:
    - Blazing performance for data operations and rendering.
    - Scripts remain lightweight, stateless, and instantly swappable.
- **Negative**:
    - Must expose ergonomics-friendly Rust API handles to the script engine.
