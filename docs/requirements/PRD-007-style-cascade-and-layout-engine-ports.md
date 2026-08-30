# PRD-007: Style Cascade and Layout Engine Ports

- **Status**: Proposed
- **Author**: Core Architecture Team
- **Date**: 2026-08-28
- **Target Release**: v0.5

---

## 1. Executive Summary

CSS **parsing** (tokenizer, selector syntax, rule sets) stays native Rust in `core/css`. The **cascade resolution** and
**layout** stages of the pipeline are exposed as replaceable ports, so an engine developer can substitute a custom
specificity/inheritance resolver or a custom layout algorithm — in Rust, or as a `.rhai` / Wasm adapter driven through
`RuntimeEngine` — without modifying `core/dom`, `core/graphics`, or any consumer. This PRD conforms to the Replaceable
Port Contract of `ADR-0011` and realises the goals stated in `PRD-001:27` and `PRD-001:62`.

---

## 2. Problem Statement

1. Cascade and layout are the policy-heavy stages of `HtmlStream → DomTree → StyledTree → LayoutBoxTree → DisplayList`
   (`ADR-0010:114-117`), and are the explicit target of the "swap the algorithm" goals — yet the pipeline exposes no
   seam there today.
2. A naive seam built from per-node FFI callbacks would violate the `<10μs` per-hook budget (`PRD-001:96`) and Object
   Calisthenics rule 3 (`ADR-0010:131`) in the hot path.
3. Without a frozen boundary aggregate, any port trait written now is rewritten when `StyledTree` changes.

---

## 3. Architecture & Port Specifications

### 3.1 Boundary aggregates (owned by `core/css`, `#[non_exhaustive]`, versioned)

- `DomSnapshot` — an immutable, read-only projection of `DomTree` (elements, attributes, tree shape). No `core/dom`
  internal type leaks; it is produced by an explicit mapping function.
- `StyleSheetSet` — parsed, ordered rules with origin (`UserAgent`, `User`, `Author`). Produced by the native Rust
  parser; not replaceable in this PRD.
- `StyledTree` — computed value per node after the cascade.
- `LayoutBoxTree` — boxes with resolved geometry, ready for `DisplayList` generation.

### 3.2 `CascadeResolver` trait (`css/application/ports.rs`)

```rust
pub trait CascadeResolver: Send + Sync {
    fn resolve(&self, dom: &DomSnapshot, sheets: &StyleSheetSet)
        -> Result<StyledTree, CssError>;
}
```

Whole-tree in, whole-tree out — coarse granularity is **mandated**, not optional. The resolver is pure and
deterministic: identical inputs produce an identical `StyledTree`.

### 3.3 `LayoutEngine` trait (`css/application/ports.rs`)

```rust
pub trait LayoutEngine: Send + Sync {
    fn layout(&self, styled: &StyledTree, constraints: &ViewportConstraints)
        -> Result<LayoutBoxTree, CssError>;
}
```

### 3.4 Script-driven adapters

When an adapter is a `.rhai` or Wasm script it runs through `RuntimeEngine` (`PRD-002`). This requires `DomSnapshot` and
`StyledTree` to be registered as engine types (`CustomType`), which depends on `C-03`. A script adapter is granted
`DOM_READ | GRAPHICS_DRAW` and **never** `DOM_MUTATE`, matching the CSS/Style profile of `PRD-003:56`.

### 3.5 Reference implementations

The built-in Rust cascade resolver and the built-in flow-plus-Flexbox layout engine are themselves adapters behind these
ports — the contract is dogfooded, not bypassed for the default path.

---

## 4. Requirements & Invariants

1. **No per-node callbacks** cross the seam; the unit of exchange is the whole tree.
2. **Determinism**: the same `DomSnapshot` + `StyleSheetSet` yields a byte-identical `LayoutBoxTree`, verified by golden
   images on `SoftwareCpuBackend` and by rectangle-assertion tests (`roadmap §5`).
3. **Fallback**: a script adapter that errors, panics, or exceeds its instruction budget falls back to the built-in Rust
   adapter, and the page still renders (`PRD-003:66-69`).
4. **No foreign types**: no `core/dom` or `core/graphics` internal type appears in a port signature or a boundary
   aggregate.
5. **Contract compliance**: this port satisfies all seven items of `ADR-0011`, including the `no-script` feature (Rust
   adapters only) and the `css-conformance` target.

---

## 5. Acceptance Criteria

- [ ] `CascadeResolver`, `LayoutEngine`, and the four boundary aggregates defined in `core/css`, frozen at integration
      point `I3`.
- [ ] Built-in Rust cascade and layout adapters pass the `css-conformance` suite.
- [ ] A mock `CascadeResolver` swaps in and changes computed styles **without changing** `core/dom` or `core/graphics`.
- [ ] A `.rhai` cascade adapter alters a computed property and the screen repaints, with capability limited to
      `DOM_READ | GRAPHICS_DRAW`.
- [ ] A script adapter that panics falls back to the built-in resolver and the page still renders.
- [ ] `core/css` builds and tests with `--no-default-features` (feature `no-script`), using only Rust adapters.
- [ ] Determinism test: 100 repeated runs of the same input produce the identical `LayoutBoxTree`.
