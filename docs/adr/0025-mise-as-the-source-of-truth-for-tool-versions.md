# ADR-0025: mise Is the Source of Truth for Tool Versions; Node Is Removed

- **Status**: Accepted (supersedes the tooling choices of ADR-0008: pnpm, prettier, markdownlint-cli2)
- **Deciders**: Architecture Team
- **Date**: 2026-10-05

---

## Context and Problem Statement

Issue #30 asks for mise to be the source of truth of tool versions. Tool versions were scattered across
`rust-toolchain.toml`, `package.json` (`packageManager`), inline pins in `.github/workflows/ci.yml` and the `justfile`.
CI also compiled `cargo-deny`, `cargo-fuzz`, `arch-lint-cli`, `cargo-geiger` and `cargo-llvm-cov` from source with
`cargo install` on every cache miss. Node existed only to run prettier and markdownlint on Markdown.

## Decision

1. `mise.toml` pins every developer/CI tool. CI installs them with `jdx/mise-action` (`install_args` per job, so a job
   pays only for its own tools). Prebuilt binaries are preferred; a tool without one is declared through mise's `cargo:`
   backend and compiled once, cached by the action.
2. **Exception**: the Rust toolchain stays in `rust-toolchain.toml` (rustup, cargo and IDEs read it natively).
3. `rumdl` replaces prettier and markdownlint-cli2 for Markdown format and lint (`.rumdl.toml`). Node, pnpm,
   `package.json`, `pnpm-lock.yaml` and their configs are removed; the former `package.json` scripts live in the
   `justfile`.
4. `lefthook` is installed by mise; `just setup` runs `mise install`.
5. All existing `.md` files were reformatted once with `rumdl fmt` (separate commit).

## Consequences

- Positive: one place for versions; no node toolchain; faster CI (no from-source builds for tools with prebuilt
  releases).
- Negative: contributors need `mise`; `cargo:`-backend tools still compile on a cold cache; rumdl's formatting differs
  slightly from prettier's, accepted via the one-time reformat.
