# 📚 Alloy Documentation

Welcome to the technical documentation and architectural specifications for **Alloy**, the infinitely malleable, modular
web browser.

---

## 🗂️ Documentation Structure

```text
docs/
├── README.md                           # This index
├── architecture/                       # System Architecture & Design
│   └── overview.md                     # High-level architecture, C4 diagrams, and core patterns
├── requirements/                       # Product & System Requirements (PRDs)
│   ├── PRD-001-alloy-core-system.md    # Overall browser system requirements
│   ├── PRD-002-abstract-runtime-engine.md # Abstract engine & Rhai integration requirements
│   ├── PRD-003-script-isolation-and-sandboxing.md # Context isolation & capability security
│   ├── PRD-004-hot-reload-subsystem.md # Runtime script hot-reloading requirements
│   └── PRD-005-graphics-and-gpu-rendering.md # Vulkan & OpenGL graphics pipeline requirements
└── adr/                                # Architecture Decision Records (MADR format)
    ├── README.md                       # ADR index & decision log
    ├── 0001-record-architecture-decisions.md
    ├── 0002-abstract-runtime-engine-and-rhai-backend.md
    ├── 0003-skeleton-and-muscle-domain-separation.md
    ├── 0004-hierarchical-capability-sandboxing-and-fault-isolation.md
    ├── 0005-atomic-hot-reloading-with-stateless-script-swaps.md
    ├── 0006-cargo-workspace-modular-crate-structure.md
    ├── 0007-spdd-methodology-and-reasons-canvas-integration.md
    ├── 0008-git-hooks-and-code-quality-tooling.md
    ├── 0009-vulkan-rendering-with-opengl-fallback.md
    └── 0010-clean-architecture-ddd-and-object-calisthenics.md
```

---

## 🚀 Key Architectural Pillars

1. **Skeleton and Muscle Model**: Rust defines strictly-typed, memory-safe data structures ("Skeleton"); swappable
   runtime scripts define all behavior and processing ("Muscle").
2. **Abstract Runtime Engine**: Core domains do not depend on a specific script interpreter. A trait-based engine
   interface supports Rhai as the primary engine and allows future engines (e.g. QuickJS, WebAssembly).
3. **Clean Architecture & Structural DDD**: Core crates isolate domain invariants and value objects with inward
   dependencies (`domain/` ➔ `application/` ➔ `infrastructure/`), using immutable aggregate pipelines.
4. **Rust-Idiomatic Object Calisthenics**: Strict elimination of primitive obsession via Newtypes (`NodeId`, `Px`,
   `Color`), first-class collections, zero naked strings, and guard clauses.
5. **Hierarchical Capability Sandboxing**: User scripts operate within isolated scopes with strictly defined
   capabilities (e.g. UI logic cannot access raw TCP sockets).
6. **Zero-Downtime Hot Reloading**: Dynamic script updates reload cleanly at runtime without restarting the browser or
   losing underlying Rust domain state.
7. **Multi-Tier GPU Graphics**: Hardware-accelerated Vulkan rendering (`vulkano`) with automatic runtime fallback to
   OpenGL (`glow`) and CPU software rendering.
8. **SPDD-Driven Engineering**: All feature development follows the Structured Prompt-Driven Development (SPDD)
   methodology, using `docs/requirements/` as authoritative input into REASONS-Canvas generation.
