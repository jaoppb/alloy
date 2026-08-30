# ADR-0011: Replaceable Subsystem Ports and Conformance Contract

- **Status**: Proposed
- **Deciders**: Architecture Team
- **Date**: 2026-08-28

---

## Context and Problem Statement

`ADR-0002` established how one seam is made swappable: the browser muscle engine, via the `RuntimeEngine` /
`ExecutionContext` traits in `core/engine`. `ADR-0009` did the same, ad hoc, for graphics via `RenderBackend`. `PRD-001`
promises more of the same for users and engine developers: _"swapping layout algorithms or intercepting requests"_
(`PRD-001:27`) and _"Swapping the HTML parser or CSS cascade resolver with custom Rhai/Wasm implementations"_
(`PRD-001:62`).

There is no single, enforced definition of what makes a subsystem replaceable. Each new seam is reinvented, the core has
no mechanical proof that it stayed decoupled from a concrete implementation, and the specification drifts from the code
— the exact failure mode the implementation roadmap calls out as its highest documentation risk. A fork author or an
alternative-backend author currently has no stable target to build against.

How should Alloy define a replaceable subsystem port so that any future implementation — a fork, an alternative Rust
backend, or a script/Wasm adapter — can substitute a subsystem without patching upstream crates?

---

## Decision Drivers

- One repeatable definition of "replaceable port", identical across every `core/*` crate.
- An alternative implementation must replace a subsystem **without modifying** the domain crate or its consumers.
- The core must **prove in CI** that it did not couple to any concrete adapter.
- Preserve Skeleton and Muscle (`ADR-0003`): mechanism ports are compiled in; policy may additionally be driven at
  runtime through `RuntimeEngine`.
- Do not regress the `<10μs` per-hook budget (`PRD-001:96`) or Object Calisthenics (`ADR-0010:123-137`).
- Stay inside the Clean Architecture layering of `ADR-0010:54-74` (`domain/` → `application/` → `infrastructure/`).

---

## Considered Options

- **Option 1**: A cross-cutting **Replaceable Port Contract** — a mandatory seven-part definition every port PRD must
  satisfy, plus a per-port conformance test suite, an in-tree reference adapter, and a `no-<adapter>` build feature that
  exercises the domain crate with no real adapter attached.
- **Option 2**: Keep defining ports ad hoc, per subsystem, with no shared rules (status quo of `ADR-0002` and
  `ADR-0009`).
- **Option 3**: A universal dynamic plugin ABI (native `.so`/`.dylib` or Wasm) so every subsystem is swappable at
  runtime without recompiling.

---

## Decision Outcome

Chosen option: **Option 1 (Replaceable Port Contract)**.

Option 2 guarantees drift and reinvention. Option 3 is rejected for the same reasons `ADR-0003` rejected static plugin
crates and `PRD-002` rejected C/C++ engines: Rust has no stable ABI, a Wasm plugin host is a project of its own, and
neither is needed — compile-time adapter selection plus runtime `.rhai` policy already covers the stated use cases.

### The Contract

Every subsystem declared replaceable MUST define all seven of the following. A port PRD is incomplete until each item is
specified; a CI gate is incomplete until items 6 and 7 are enforced.

1. **Seam PRD** — a `docs/requirements/PRD-*.md` that names the seam, the **variation model** (what legitimately differs
   between implementations), and the **threat model** (trusted author vs hostile third party). It follows the structure
   of the existing PRDs and ends in `- [ ]` acceptance criteria.
2. **Port traits in `application/ports.rs`** — associated types only (`type Compiled`, `type Error`, …); **zero
   concrete-adapter types** in any signature; `Send + Sync` wherever the aggregate crosses a thread boundary; no generic
   method that would make the trait non-object-safe unless a companion object-safe form is also provided.
3. **Boundary aggregates** — the immutable input and output types are **owned by the domain crate**, marked
   `#[non_exhaustive]`, carry an explicit schema version, and never expose a foreign crate's type. Conversion across the
   seam is an explicit mapping function or DTO (`ADR-0010:114-119`), never a re-exported adapter type.
4. **Typed error** — exactly one `enum <Port>Error` per port, with source-location metadata (line/column/offset where
   meaningful). Adapter-specific errors are mapped into it. No `Box<dyn Error>` and no adapter error type crosses the
   seam.
5. **Lifecycle and concurrency contract** — written documentation of: who owns durable state (always the Skeleton,
   `ADR-0003`), threading model, re-entrancy and suspension, cancellation, and resource ceilings (instruction/step
   budgets, memory). Fault behaviour references the trapping/fallback model of `PRD-003:62-70`.
6. **Conformance suite + reference adapter** — an in-tree `<crate>-conformance` test target that any adapter must pass;
   a reference or mock adapter kept in the repository; and a `no-<adapter>` Cargo feature that builds and tests the
   domain crate with **no real adapter linked**. This generalises the `no-engine` feature of `PRD-001:99` (N-04) to
   every port.
7. **Frozen-API milestone** — the port's public surface is frozen at a named roadmap integration point (for example `I3`
   freezes `core/dom`). After the freeze, any change to a boundary aggregate requires a schema-version bump and a
   migration note in the port PRD.

### Mechanism vs. Policy

- **Mechanism ports** (HTML tokenizer, CSS cascade, layout, rasterizer, content JS engine) are swapped **at compile
  time** by selecting an adapter crate. They are not runtime-loadable.
- **Policy ports** (pipeline ordering, event routing, request interception) MAY additionally be driven **at runtime**
  through `RuntimeEngine`, once their boundary aggregates are registered as engine types (`C-03`). The adapter is then a
  `.rhai` script and is subject to the capability profile of `PRD-003:55-58`.
- **Runtime dynamic loading of native code** stays rejected, consistent with `ADR-0003` (Option 3) and `PRD-002`.

### Ports governed by this contract

| Port                                   | Crate           | Kind               | Seam PRD                              | Freeze point |
| -------------------------------------- | --------------- | ------------------ | ------------------------------------- | ------------ |
| `RuntimeEngine` / `ExecutionContext`   | `core/engine`   | Mechanism          | `PRD-002` (retrofit to this contract) | `F1`         |
| `RenderBackend`                        | `core/graphics` | Mechanism          | `PRD-005` (retrofit to this contract) | `F4`         |
| `ContentScriptEngine` / `HostBindings` | `core/js`       | Mechanism          | `PRD-006`                             | `I3`         |
| `CascadeResolver` / `LayoutEngine`     | `core/css`      | Mechanism + policy | `PRD-007`                             | `I3`         |
| `TokenSink` / `TreeSink`               | `core/html`     | Mechanism + policy | `PRD-008`                             | `I3`         |

`PRD-002` and `PRD-005` are retroactively instances of this contract and gain a `no-<adapter>` feature and a conformance
target when their crates are implemented. The concrete selection of the first content-JS engine (`boa` versus
alternatives, argued in the implementation roadmap) is a separate decision and will be recorded in **ADR-0012**.

### Consequences

- **Positive**:
    - New ports are checklist-driven, not bespoke; review has one rubric.
    - CI mechanically proves each domain crate is decoupled from its adapter (`no-<adapter>` build + mock swap test).
    - Forks and alternative backends have a stable, versioned target.
    - The Skeleton/Muscle line is preserved: mechanism compiled in, policy optionally scripted, native plugins still
      out.
- **Negative**:
    - Every port carries an extra conformance crate and an extra column in the CI matrix.
    - Boundary aggregates must be versioned, adding mapping boilerplate (`ADR-0010` already accepts this cost for
      cross-crate DTOs).
    - Three new PRDs (`PRD-006`, `PRD-007`, `PRD-008`) must be ratified and kept in sync with the code that implements
      them.
