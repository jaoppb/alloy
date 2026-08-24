# ADR-0010: Clean Architecture, Domain-Driven Design, and Object Calisthenics

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-08-23

---

## Context and Problem Statement

Alloy is a highly modular browser engine comprising 11 Cargo workspace crates. To ensure maintainability, testability,
and zero domain pollution across crates, we must define clear architectural layers, domain boundaries, and code-level
design rules. Furthermore, we must evaluate and position **Feature-Sliced Design (FSD)** alongside **Clean
Architecture** and **Domain-Driven Design (DDD)** within the "Skeleton and Muscle" model.

---

## Decision Drivers

- Maintain strict separation of concerns across core systems and runtime interpreters.
- Eliminate "stringly-typed" and primitive obsession bugs in domain representations.
- Support high-throughput compiler-like transformation pipelines (HTML ➔ DOM ➔ CSS Cascade ➔ Layout ➔ DisplayList).
- Establish unambiguous folder and module conventions across all Cargo crates.

---

## Considered Architectural Paradigms

### 1. Clean Architecture + DDD (Selected for Core Crates)

- Decomposes subsystems by **Bounded Contexts** and inward dependency layers (`domain/` ➔ `application/` ➔
  `infrastructure/`).
- Optimal for systems-level pipelines, data structures, and deterministic transformations.

### 2. Feature-Sliced Design (FSD) (Scoped to UI/DevTools)

- Decomposes codebase by user-facing functional slices (`app`, `pages`, `widgets`, `features`, `entities`, `shared`).
- Ideal for user interface shells and DevTools panels, but ill-fitted for high-frequency systems-level compiler
  pipelines (like HTML parsers or CSS cascades).

---

## Decision Outcome

Chosen option: **Clean Architecture + DDD per Crate with Hybrid FSD for UI/DevTools, Mechanism vs. Policy Separation,
and Rust-Idiomatic Object Calisthenics**.

---

## 1. Clean Architecture Crate Layout

Every Cargo crate in `core/*` follows an inward dependency structure:

```text
crate_root/
├── Cargo.toml
└── src/
    ├── lib.rs                 # Public facade / exported ports
    ├── domain/                # Innermost: Entities, Value Objects, Domain Errors, Invariants
    │   ├── entity.rs          # State + Identity + Invariant-protecting methods
    │   ├── value_object.rs    # Immutable Newtypes (e.g. NodeId, TagName, Px)
    │   └── error.rs           # Domain-specific typed error enums
    ├── application/           # Middle: Pipeline orchestrators, use cases, ports (traits)
    │   ├── service.rs         # Domain service orchestration
    │   └── ports.rs           # Trait interfaces for external infrastructure
    └── infrastructure/        # Outermost: Concrete adapters (Vulkano, Sockets, Rhai bindings)
        └── adapter.rs         # Implements ports defined in application/
```

### Dependency Invariant

- `domain` depends on **nothing** (zero external framework dependencies, zero I/O).
- `application` depends only on `domain`.
- `infrastructure` depends on `application` and `domain`.

---

## 2. Structural DDD (Skeleton) vs. Behavioral DDD (Muscle)

In Alloy's "Skeleton and Muscle" architecture, DDD is adapted to a **Mechanism vs. Policy** separation:

```text
┌────────────────────────────────────────────────────────────────────────┐
│                        Alloy Architectural Split                       │
├────────────────────────────────────────────────────────────────────────┤
│  THE MUSCLE (Runtimes / Rhai / JS Scripts)                             │
│  ► Owns BEHAVIORAL DOMAIN & POLICIES                                   │
│    - Event reactions ("on tab clicked", "on network request")          │
│    - Routing, user workflows, navigation rules                         │
│    - Script-driven pipeline composition                                │
├────────────────────────────────────────────────────────────────────────┤
│  THE SKELETON (Rust Core)                                              │
│  ► Owns STRUCTURAL DOMAIN & MECHANISMS (Invariants & Capabilities)     │
│    - Structural integrity (e.g. DOM tree acyclicity, valid pointers)   │
│    - Memory safety, raw computation (rasterization, tokenization)      │
│    - Capability sandbox gates (Least Privilege enforcement)            │
│    - Ubiquitous Language & Strong Value Objects (Object Calisthenics)  │
└────────────────────────────────────────────────────────────────────────┘
```

1. **Rust Core owns Structural DDD (Mechanisms & Invariants)**:
    - Enforces tree hierarchy invariants (e.g., node cannot be its own ancestor).
    - Enforces memory boundaries, capability gates, and strong value type validations.
    - Does **not** hardcode arbitrary user-facing business policies.
2. **Runtime Engine owns Behavioral DDD (Policies & Workflows)**:
    - Scripts define the behavioral rules, event listeners, and user customization workflows.

---

## 3. Aggregate Pipelines & Cross-Crate Communication

1. **Autonomous Bounded Contexts**: Each Cargo crate represents a distinct Bounded Context with its own Ubiquitous
   Language (`dom`, `html`, `css`, `graphics`, `window`, `network`, `engine`, `rhai-runtime`, `js`).
2. **Immutable Aggregate Pipelines**: Subsystems interact by consuming an immutable Aggregate and producing a verified,
   immutable Aggregate for the next stage:
    - `HtmlStream` ➔ `DomTree` Aggregate ➔ `StyledTree` Aggregate ➔ `LayoutBoxTree` Aggregate ➔ `DisplayList` Value
      Object ➔ `RenderBackend` Presenter.
3. **Anti-Corruption Boundaries**: Conversions between crates happen through explicit domain mapping functions or Data
   Transfer Objects (DTOs), preventing type leaking.

---

## 4. Rust-Idiomatic Object Calisthenics (9 Rules)

Alloy enforces strict Object Calisthenics adapted to Rust:

| Rule  | Calisthenics Rule                         | Rust Implementation & Invariants                                                                                                                   |
| ----- | ----------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| **1** | **One level of indentation per function** | Use `?` operator, iterator combinators (`map`, `and_then`), and helper extraction.                                                                 |
| **2** | **Don't use the `else` keyword**          | Use early returns, pattern matching (`match`), `if let`, and `unwrap_or_else`.                                                                     |
| **3** | **Wrap all primitives and strings**       | **Zero naked primitives** in domain models. Use strong Newtypes: `NodeId(u32)`, `TagName(String)`, `Px(f32)`, `Color(u32)`.                        |
| **4** | **First-class collections**               | Wrap standard collections with domain invariants: `Children(Vec<NodeId>)`, `RuleSet(Vec<CssRule>)`, `HeaderMap(HashMap<HeaderName, HeaderValue>)`. |
| **5** | **One dot per line**                      | Respect Law of Demeter. Never do deep property navigation (`a.b.c.d`). Builder method chains are permitted.                                        |
| **6** | **Don't abbreviate names**                | Use full, descriptive names from the Ubiquitous Language (`element_identifier`, not `el_id`).                                                      |
| **7** | **Keep entities small**                   | Structs and modules should remain focused (<100 lines per struct, single responsibility).                                                          |
| **8** | **No public mutable fields**              | Struct fields are private (`pub(crate)` or private). State mutations occur through invariant-validating methods.                                   |
| **9** | **Encapsulate state & behavior**          | Do not create anemic data structures for entities; bundle structural invariants with the data.                                                     |

---

## 5. Feature-Sliced Design (FSD) Comparison & Hybrid Placement

```text
┌────────────────────────────────────────────────────────────────────────┐
│                      Alloy Architecture Topology                       │
├────────────────────────────────────────────────────────────────────────┤
│  UI / Tooling Layer (Feature-Sliced Design)                            │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ devtools / extension / browser-ui                                │  │
│  │ ├── app/ (shell & state bootstrap)                              │  │
│  │ ├── features/ (inspector, network-monitor, script-hot-reloader) │  │
│  │ ├── widgets/ (tab-bar, url-omnibox, console-panel)               │  │
│  │ └── shared/ (ui-primitives, theme-tokens)                        │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                  │                                     │
│                                  ▼ Uses Ports / Trait Interfaces       │
│  Core Browser Engines (Clean Architecture + DDD Bounded Contexts)      │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ core/engine, core/dom, core/html, core/css, core/graphics, ...  │  │
│  │ ├── domain/ (Entities, Strong Newtypes, Invariants)              │  │
│  │ ├── application/ (Aggregate Pipelines, Domain Services)         │  │
│  │ └── infrastructure/ (Adapters: Vulkano, Glow, Sockets, Rhai)    │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
```

---

## Consequences

- **Positive**:
    - Zero primitive obsession bugs thanks to strong Rust Newtypes.
    - 100% testable domain logic independent of GPU drivers, filesystems, or script engines.
    - Clean division between structural safety in Rust and policy customization in scripts.
    - Explicit aggregate pipelines make browser execution deterministic and traceable.
    - FSD provides modular scalability for complex UI panels without polluting systems-level core crates.
- **Negative**:
    - Requires writing Newtype boilerplate and constructor validation logic (mitigated by derive macros).
