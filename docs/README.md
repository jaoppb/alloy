# 📚 Alloy Documentation

Welcome to the technical documentation and architectural specifications for **Alloy**, the infinitely malleable, modular
web browser.

---

## 🗂️ Documentation Structure

```text
docs/
├── README.md                           # This index
├── architecture/                       # System Architecture & Design
│   ├── overview.md                     # High-level architecture, C4 diagrams, and core patterns
│   ├── runtime-engine-port-contract.md # ADR-0011 contract record for the RuntimeEngine port
│   ├── style-cascade-port-contract.md  # ADR-0011 contract record for the CascadeResolver/LayoutEngine/TextMeasurer port
│   ├── http-transport-port-contract.md # ADR-0011 contract record for the HttpTransport/RequestPolicy port (not yet frozen)
│   └── window-system-port-contract.md  # ADR-0011 contract record for the WindowSystem/Presenter port (not yet frozen)
├── requirements/                       # Product & System Requirements (PRDs)
│   ├── PRD-001-alloy-core-system.md    # Overall browser system requirements
│   ├── PRD-002-abstract-runtime-engine.md # Abstract engine & Rhai integration requirements
│   ├── PRD-003-script-isolation-and-sandboxing.md # Context isolation & capability security
│   ├── PRD-004-hot-reload-subsystem.md # Runtime script hot-reloading requirements
│   ├── PRD-005-graphics-and-gpu-rendering.md # Vulkan & OpenGL graphics pipeline requirements
│   ├── PRD-006-web-content-javascript-runtime-port.md # Replaceable content JS engine port
│   ├── PRD-007-style-cascade-and-layout-engine-ports.md # Replaceable CSS cascade & layout ports
│   ├── PRD-008-html-tokenizer-and-tree-sink-ports.md # Replaceable HTML tokenizer & tree sink ports
│   ├── PRD-009-network-transport-and-request-policy-port.md # Replaceable HTTP transport & request-policy ports
│   └── PRD-010-window-system-and-presenter-port.md # Replaceable window-system & presenter ports
├── reports/                            # Technical Reports (analysis, audits, roadmaps)
│   ├── ROADMAP-IMPLEMENTACAO-V1.md     # Phased roadmap from bootstrap to the v1.0 release
│   ├── IMPLEMENTACAO-DETALHADA-V0-1.md # Detailed F0+F1+F2 implementation plan for v0.1
│   ├── IMPLEMENTACAO-DETALHADA-V0-2.md # Detailed F3+F6+I1 implementation plan for v0.2
│   ├── IMPLEMENTACAO-DETALHADA-V0-3.md # Detailed F4+F5+I2 implementation plan for v0.3
│   ├── IMPLEMENTACAO-DETALHADA-V0-5.md # Detailed implementation plan for v0.5 (B0-B5, C0-C2, EE, I2, I4, M, P)
│   ├── V0-5-PROGRESSO-E-PENDENCIAS.md  # v0.5 progress snapshot with file:line evidence
│   └── VIOLACAO-N02-UNSAFE-NO-RHAI.md  # Audit: PRD-001:97 vs. the unsafe in rhai's binding seam (closed by ADR-0018)
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
    ├── 0010-clean-architecture-ddd-and-object-calisthenics.md
    ├── 0011-replaceable-subsystem-ports-and-conformance-contract.md
    ├── 0013-object-safe-runtime-engine-companion.md
    ├── 0014-structured-logging-with-tracing.md
    ├── 0015-typed-errors-with-thiserror.md
    ├── 0018-unsafe-by-threat-surface.md
    └── 0019-single-event-loop-owns-the-main-thread.md
```

`docs/v0-5-handoff/` holds the self-contained, per-phase handoff package for the v0.5 campaign (B4/B5/X/I2/M/I4/P) —
each file there is meant to be opened on its own by a fresh session, without reading the rest of this documentation tree
first.

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
