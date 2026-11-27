# ADR-0029: A Nix Flake Is the Required Development Environment

- **Status**: Accepted (supersedes decisions 1 and 4 of
  [ADR-0025](0025-mise-as-the-source-of-truth-for-tool-versions.md))
- **Deciders**: Architecture Team
- **Date**: 2026-11-21

---

## Context and Problem Statement

ADR-0025 made `mise.toml` the source of truth for tool versions, with two gaps: the Rust toolchain lived in
`rust-toolchain.toml` instead, and native dependencies (the `winit`/`softbuffer` system libraries, the linker, a C
compiler) were not pinned at all. Several tools were `latest`, so two machines could run different versions. CI repeated
a per-job `jdx/mise-action` install. Issue #152 (epic #154) asks for one lockfile that pins everything.

## Decision

1. **A Nix flake (`flake.nix` + `flake.lock`) is the required environment** for building, testing and running Alloy on
   Linux and macOS. `nix develop`, or direnv's `use flake` (`.envrc`), is the only setup path. mise is removed.
2. **Re-affirming ADR-0025 decision 2:** `rust-toolchain.toml` stays the one place the Rust channel is declared. The
   flake reads it through `oxalica/rust-overlay` (`fromRustupToolchainFile`) and adds `llvm-tools-preview`, so rustup
   users, IDEs and the Windows CI job agree with the flake.
3. **Tools** come from nixpkgs (`just`, `lefthook`, `cargo-deny`, `cargo-llvm-cov`, `cargo-geiger`, `cargo-fuzz`, `jq`)
   except `arch-lint-cli`, which the flake fetches as the pinned upstream static musl binary on x86_64-linux (the CI
   platform, where the source build costs about 1m49s cold; upstream ships no musl build for aarch64-linux) and builds
   from crates.io at the same pinned version elsewhere (it is not packaged), with the prebuilt hash taken from the
   release's `.sha256` asset and checked against GitHub's digest, and `rumdl`, which the flake fetches as the pinned
   upstream release binary (static musl on Linux): nixpkgs' `rumdl` is older than the version whose formatting CI
   enforces, and compiling it from source took about 10 minutes and 18 GB of RAM, which timed out the cold Markdown CI
   job.
4. **Shells.** `default` carries every tool; `fuzz` carries a nightly toolchain and `cargo-fuzz`; slim shells
   (`markdown`, `rust`, `supply-chain`, `unsafe-audit`, `arch`, `coverage`) let each CI job pay only for its own tools.
   Every shell exports `ALLOY_DEV_SHELL`, which `just setup` checks.
5. **Windows is the one exception.** Nix does not run natively there, so the `windows-2022` CI job stays Nix-free
   (rustup reading `rust-toolchain.toml`) and Windows developers use WSL2 inside the flake. `justfile` recipes stay
   tool-agnostic; only `just setup` guards on Linux/macOS.
6. ADR-0025's decisions 3 (rumdl replaces prettier/markdownlint, no Node) and 5 (one-time reformat) stay in force.

## Consequences

- Positive: one lockfile for the compiler, tools and native libraries; each worktree gets an identical shell; CI jobs
  and local runs use the same environment.
- Negative: contributors on Linux and macOS must install Nix; the first entry on aarch64-linux and macOS builds
  `arch-lint-cli` from source unless cached; `rumdl` and the x86_64-linux `arch-lint-cli` are downloaded binaries, so
  their hashes must be updated with each version bump; the Windows job is a second, rustup-based path that can drift
  from the flake.
- Follow-ups: mold (#153) and sccache (#151) add their packages to this flake; `arch-lint-cli` moves to 0.11 with #155.
