# ADR-0011: Typed Domain Errors with thiserror

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-08-26

---

## Context and Problem Statement

Alloy enforces strict domain modeling and typed error handling across all core workspace crates (`core/dom`, `core/css`,
`core/engine`, `core/graphics`, `core/html`). Previously, each crate manually implemented `std::fmt::Display` and
`std::error::Error` for its domain error enums (`DomError`, `CssError`, `EngineError`, `GraphicsError`, `HtmlError`).

Manual error implementations suffer from:

1. High boilerplate and risk of syntax/formatting drift between enum variants and message strings.
2. Inconsistent support for nested error chaining and causal tracking (`std::error::Error::source`).
3. Manual implementation of error conversions (`From<T>`), leading to ad-hoc error propagation patterns.

How should domain errors be implemented and standardized across the workspace?

---

## Decision Drivers

- Standardize error formatting and display strings across all domain crates.
- Automate causal error chaining (`#[source]` and `#[from]`) without handwritten boilerplate.
- Ensure compile-time validation of error format strings.
- Maintain workspace license compliance (dual `MIT OR Apache-2.0`).
- Clarify the dependency policy of `core/engine`: lightweight macro and type definition dependencies (`thiserror`,
  `bitflags`) are permitted in core abstraction crates.

---

## Considered Options

- **Option 1**: Use `thiserror` (v2.x) derive macros across all workspace crates defining domain errors.
- **Option 2**: Retain manual `impl fmt::Display` and `impl std::error::Error` in all crates.
- **Option 3**: Adopt `thiserror` only outside `core/engine` to artificially maintain a zero-dependency declaration in
  `core/engine`.

---

## Decision Outcome

Chosen option: **Option 1**, because:

1. `thiserror` is the de-facto standard in the Rust ecosystem for typed library and domain errors.
2. It provides compile-time checking of formatted arguments and automatically implements `std::error::Error` and
   `source()`.
3. The `#[from]` attribute transparently implements standard error conversions across crate boundaries (e.g.
   `From<DomError> for CssError`).
4. `thiserror` operates purely at compile-time as a procedural derive macro without runtime overhead, binary bloat, or
   external I/O coupling.
5. In conjunction with this decision, `CLAUDE.md` is updated to accurately reflect that `core/engine` allows pure
   type/macro dependencies (`thiserror`, `bitflags`) while strictly confining external I/O adapters to its
   `infrastructure/` module.

### Consequences

- **Positive**:
    - Eliminated manual `fmt::Display` boilerplate across 5 core crates.
    - Consistent error formatting and transparent causal propagation via `#[from]`.
    - Compile-time format string validation.
    - License compliant (`MIT OR Apache-2.0`).
- **Negative**:
    - Adds a compile-time build dependency on `thiserror` across workspace crates.
