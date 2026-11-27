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
   upstream release binary (static musl on Linux), pinned by four per-platform hashes (x86_64/aarch64 Linux and Darwin):
   nixpkgs' `rumdl` is older than the version whose formatting CI enforces, and compiling it from source took about 10
   minutes and 18 GB of RAM, which timed out the cold Markdown CI job. CI exercises only the x86_64-linux binary; the
   aarch64-linux and both Darwin hashes were verified by hand with `nix store prefetch-file --hash-type sha256`
   (2026-11-22, v0.3.5). The bump procedure is a comment in `flake.nix`.
4. **Shells.** `default` carries every tool; `fuzz` carries a nightly toolchain and `cargo-fuzz`; slim shells
   (`markdown`, `rust`, `supply-chain`, `unsafe-audit`, `arch`, `coverage`) let each CI job pay only for its own tools.
   Every shell exports `ALLOY_DEV_SHELL`, which `just setup` checks. The `fuzz` shell's nightly comes from
   `selectLatestNightlyWith`, so a `nix flake update` cannot land on a nightly with a missing component.
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

- Cache budget: Actions allows 10 GB per repository, shared by the Nix store entries (one per shell and `flake.lock`
  generation) and the cargo `target/` caches. On a key miss, cache-nix-action v7 runs `nix store gc --max <cap>` just
  before saving (on a hit it neither collects nor saves); the garbage collector deletes dead paths in store-directory
  order, not oldest first. The nix-shell action therefore roots the shell with `nix develop --profile` after the
  restore, so only paths outside the current shell closure can be deleted (verified locally: the profile's closure
  equals the shell's, 3.25 GB for `rust`, and roots its `rustc`). The caps are `gc-max-store-size-linux: 4G` and
  `gc-max-store-size-macos: 3500M`, just above the largest cached closure (3.28 GB for `unsafe-audit`, 3.25 GB for
  `rust`, ~3.0 GB on macOS). Flake inputs are not rooted (nix#6895) and may be trimmed, then refetched. Measured on
  2026-11-22 with `gh cache list`, the cache was already at 11.3 GiB (limit 10 GiB) before the caps; #355 has the
  arithmetic and #357 tracks the total.

- sccache stays optional (supersedes the epic's original "mandatory sccache" decision, #154): #151 shipped it as an
  opt-in (`RUSTC_WRAPPER=sccache`, default shell only, no `build.rustc-wrapper`). The measurement below is bound to the
  pinned sccache **0.19.0**; follow-up #365 will re-measure on sccache ≥ 0.20.0.

  Measured by #347 on a second worktree against the bar of ≥30% faster and ≥80% cache hits (two sequences, n=1 per
  configuration per sequence, same commit, 16 cores, load average 10–77 in the first sequence and 40–56 in the second):

  - **`SCCACHE_BASEDIRS` was not testable for Rust.** In 0.19.0 it applies to C/C++ only; Rust support arrived in
    v0.20.0 (#2918, #3022), together with no longer hashing `CARGO_TARGET_DIR` (#3050). So #347 is no evidence that
    basedirs fails for Rust.
  - **`--remap-path-prefix`** (confirmed on the `rustc` command line) did not help: the hit count stayed at 30/189
    (15.9%) with remap alone, basedirs alone and both together (basedirs was a no-op for Rust in 0.19.0, so these are
    effectively one configuration), and wall-clock was within noise of no sccache: warm runs 68.8–73.6 s against a
    single 71.3 s baseline in the quieter sequence; the other was too load-noisy to read.
  - **A shared `CARGO_TARGET_DIR`** makes a second worktree's no-op rebuild take 1.7–4.2 s, but its first build is
    within noise of baseline, and it is disqualified (single measurements): concurrent builds block on cargo's
    build-directory lock (a second build waited ~53 s), a `cargo clean` in one worktree wipes the others' artifacts (a
    62 s rebuild), and switching between divergent commits rebuilds the workspace (27–45 s).
  - Workspace crates built with incremental compilation are not cached by 0.19.0 (v0.20.0 adds
    `SCCACHE_ALLOW_INCREMENTAL`, #2810, #3020), nor are `bin`, `proc-macro` and `dylib` crates.

  Nothing made it default-on; CI keeps its per-job caches.

- Follow-ups: mold (#153) and sccache (#151, optional) add their packages to this flake; `arch-lint-cli` moves to 0.11
  with #155.

## Cache sharing and purge (#357)

- Shared entry: `rust`, `coverage`, `unsafe-audit`, `fuzz`, `supply-chain` and `arch` have near-identical closures
  (0.7–3.3 GB each), so on Linux they restore one `nix-Linux-toolchain-<hash>` entry (the nix-shell action's `entry`
  input). `rust-quality` is its only saver (`save: 'false'` elsewhere) and runs `nix develop .#toolchain --command true`
  so the entry holds the union of the six closures (the `toolchain` shell is built from the shells' package lists, so it
  cannot drift from them). It measures about 4.1 GB uncompressed, which is why the Linux GC cap is 5G. Markdown and
  `repo` keep their own small entries; macOS keeps its `rust` entry, saved by the macOS test job. Cache scoping already
  stops a pull request from poisoning `main`: a PR's entry is readable only by that PR's refs, while `main`'s entries
  are readable by all. Trade-off: a failing `rust-quality` leaves the entry unseeded, so a re-run is cold, and its cold
  run realises the extra closures once per flake change.
- Budget: one generation of every entry sums to 9.08 GiB (Nix 3.26 GiB, cargo `target/` 5.82 GiB), measured with
  `gh cache list` on PR #360, against the 10 GiB limit (it was 11.3 GiB before this change). About 0.9 GiB of headroom
  remains, and PR-scoped entries count too, so check this figure before adding a cache. Warm Linux restores cost 3–16 s
  more than with per-shell entries (extracting a 4.1 GB store instead of ~3.3 GB), while 11 jobs no longer save (9–24 s
  each).
- Purge: `ci/purge_stale_nix_caches.sh` deletes, on `main`, every `nix-*` entry not ending in the current
  `flake.nix`/`flake.lock`/`rust-toolchain.toml` hash. It needs `actions: write`, which is granted per job, not per
  event, so it lives in the `purge-nix-cache` job that runs only on a push to `main` and only after every other job
  succeeded and only while its commit is still the head of `main`. Every job that a pull request can reach keeps
  `contents: read`.
