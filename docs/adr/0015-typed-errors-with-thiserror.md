# ADR-0015: Typed Errors with `thiserror`

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-08-30

---

## Context and Problem Statement

Every crate needs a typed error enum (ADR-0010: no stringly-typed failures). Written by hand, each one is a
`#[derive(Debug)]` enum plus a `impl Display` `match` arm per variant plus an empty `impl std::error::Error` plus a
`impl From<...>` per wrapped source — tens of lines of mechanical code that drifts out of sync with the variants.

The `alloy` CLI landed this problem first (`AlloyError`), and `core/engine` already carries a ~45-line hand-written
`impl Display for EngineError`. We need one decision on how error types are written across the workspace.

---

## Decision Drivers

- Cut the `Display` / `Error` / `From` boilerplate without giving up a concrete, matchable enum.
- No runtime dependency and no macro expansion cost pushed onto the `core/engine` port boundary, which advertises a
  near-zero dependency surface (ADR-0002, `no-engine` gate).
- Library errors, not application errors — an enum consumers can `match`, not an opaque `anyhow` chain.

---

## Considered Options

- **Option 1**: **`thiserror`** — derives `Display` from `#[error("…")]`, `Error::source` from `#[source]` / `#[from]`,
  and the `From` impls. Compile-time only, no runtime code.
- **Option 2**: hand-written `impl Display` + `impl Error` — zero dependencies, full control, high boilerplate and drift
  risk.
- **Option 3**: `snafu` — richer (context selectors, backtraces) but a heavier API and a larger surface than needed.
- **Option 4**: `anyhow` — great for a binary's top level, wrong for library errors that callers must discriminate.

---

## Decision Outcome

Chosen option: **Option 1 (`thiserror`) for application and adapter crates; Option 2 (hand-written) for `core/engine`.**

- **`alloy`, and future application / adapter crates** derive their error enums with `thiserror`
  (`#[derive(thiserror::Error)]`, `#[error(...)]`, `#[from]`). `thiserror` is a `[workspace.dependencies]` entry that a
  crate opts into.
- **`core/engine` keeps its hand-written `impl Display` / `impl std::error::Error` for `EngineError`.** The port crate's
  whole value proposition is a tiny, auditable dependency surface (`bitflags` only, verified by the `no-engine` CI job).
  One error enum's worth of boilerplate is a price worth paying to keep that promise; the `EngineError` variants change
  rarely and are covered by conformance tests. If the port ever needs `#[from]` chains or grows past a handful of
  variants, revisit this carve-out.

### Consequences

- **Positive**:
    - Adapter / app error enums shed the `Display` / `From` boilerplate and cannot drift from their variants.
    - `core/engine` stays at one third-party dependency; the `no-engine` gate is unaffected.
- **Negative**:
    - Two conventions in the tree — but the boundary (is this `core/engine`?) is unambiguous.
    - `arch-lint`'s `require-thiserror` (AL005) is left disabled, since it would flag the deliberate `EngineError`
      carve-out.
