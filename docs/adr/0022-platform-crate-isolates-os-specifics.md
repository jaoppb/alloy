# ADR-0022: A `platform` Crate Isolates Host-OS Specifics

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-10-03

---

## Context and Problem Statement

The v0.5 fonts increment (PR #18) taught `core/graphics` to resolve a named `font-family` against the fonts installed on
the machine. To do that, `core/graphics/src/infrastructure/font/catalog.rs` grew per-OS knowledge:
`#[cfg(target_os = …)]` branches, hard-coded install paths for Linux, macOS and Windows, and environment lookups
(`HOME`, `WINDIR`, `LOCALAPPDATA`). Review asked for that OS handling to move out of the rendering crate into a shared
crate that abstracts it.

Font directories are only the first such question. Config and cache directories (hot-reload watching, F11), the user's
profile location and per-OS defaults will follow; if each subsystem answers them itself, the same `cfg` ladders and
environment variables get copied into every crate that needs one.

---

## Decision Drivers

- `core/graphics` should describe rasterization, not where Windows keeps its fonts (`ADR-0010` single responsibility).
- Per-OS behaviour must be testable on each CI runner without every subsystem knowing all three OSes.
- Any crate must be able to depend on it without creating a cycle or pulling in a subsystem.

---

## Considered Options

- **Option 1 — keep the per-OS code in `core/graphics`.** Zero churn, but the next subsystem that needs a host path
  copies the pattern.
- **Option 2 — a dependency-free `core/platform` crate (package `platform`) with a port per question.** Host knowledge
  sits behind `FontLocator` (font directories, generic-family candidates); `HostPlatform` is the adapter for the build
  target.

---

## Decision Outcome

Chosen option: **Option 2.**

- `core/platform` follows the `ADR-0010` layout: `domain/` (`GenericFamily`, `FontDirectories`, `FontCandidates`),
  `application/` (the `FontLocator` port), `infrastructure/` (`HostPlatform`).
- **The only `target_os` dispatch is the selection of the `os` module in `core/platform/src/infrastructure/mod.rs`.**
  Each per-OS module (`linux.rs`, `macos.rs`, `windows.rs`, `unsupported.rs`) exposes the same functions, so
  `HostPlatform` itself has no `cfg`.
- The crate is a **leaf**: it has no dependencies and imports no workspace crate (`arch-lint.toml`: `platform-isolated`,
  plus `deny-scope-dep` from `platform`). `#![forbid(unsafe_code)]`.
- `core/graphics` keeps its public `FontCatalog` / `GenericFamily` names (`GenericFamily` is re-exported from
  `platform`), so callers such as `alloy`'s `RuntimeFontProvider` are unchanged.

### Consequences

- **Positive**: a new host-dependent question is a new port in one crate, testable on each CI runner; subsystem crates
  stay OS-agnostic.
- **Negative**: one more workspace member, and `graphics` gains a (dependency-free) edge.
- **Neutral**: behaviour is unchanged — the paths and environment lookups moved verbatim.
