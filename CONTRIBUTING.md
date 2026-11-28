# Contributing to Alloy

Thanks for taking the time to contribute. Alloy is a Rust + Rhai browser engine built around strict architectural
contracts (ADRs), so most of what makes a change mergeable is mechanical — the checklist below gets you through it on
the first pass.

For AI coding agents (Claude Code, Gemini CLI, or otherwise): **read [`CLAUDE.md`](CLAUDE.md) first.** It is the
canonical, continuously-updated source of architecture rules, coding standards, and lessons learned from past review
cycles. This document covers the human-facing workflow around it.

## Before You Start

- Skim [`docs/adr/README.md`](docs/adr/README.md) for the architectural decisions already made — most "why is it built
  this way" questions are answered there.
- Skim [`docs/requirements/`](docs/requirements/) (`PRD-*.md`) for the product requirements a subsystem must satisfy.
- Check open issues and pull requests before starting non-trivial work, to avoid duplicating effort.

## Development Setup

```bash
git clone https://github.com/jaoppb/alloy.git
cd alloy
# Prerequisite: Nix with flakes enabled (`experimental-features = nix-command flakes`),
# https://nixos.org/download. Windows: develop inside WSL2 (ADR-0029).
nix develop  # required on Linux and Apple Silicon macOS; Intel Macs are unsupported (or `direnv allow`; review `flake.nix` changes before entering the directory): every tool, the Rust toolchain, native libraries
just setup   # checks you are in the flake shell, installs the git hooks
```

The default flake shell also provides `sccache`, off by default: `export RUSTC_WRAPPER=sccache` opts in. In the #347
measurement it only hit when absolute paths matched: with the pinned sccache 0.19.0 `SCCACHE_BASEDIRS` is not applied to
Rust, path remapping had no measurable effect, and a shared `CARGO_TARGET_DIR` blocks concurrent builds, so a second
worktree with its own `target/` gains little (#347, to be re-measured on sccache ≥ 0.20.0 in #365; see `CLAUDE.md`).

`just` (no arguments) lists every available recipe. The two you'll use constantly:

```bash
just gate                       # (inside the flake shell) full local quality gate — mirrors CI exactly
just run --script <path>        # run the alloy binary against a .rhai script
```

## Workflow

1. **Branch from `main`.** Use a descriptive branch name (`feat/…`, `fix/…`, `docs/…`, `refactor/…`).

2. **Follow the architecture.** Every crate is layered `domain/` → `application/` → `infrastructure/` (dependencies
   point inward only); domain crates never depend on `rhai` directly. See the "Architecture" section of `CLAUDE.md` for
   the full crate map and layering rules.

3. **Write to the coding standard.** Clean Code as the baseline, Object Calisthenics as the mechanically-enforced subset
   (no naked primitives, no naked `Vec`/`HashMap`, no `else`, one level of indentation per function, no
   `unwrap`/`expect`/`panic!` on a reachable path, and more — see `CLAUDE.md`). Clippy runs with `-D warnings` — a
   clippy warning is a build failure, not a suggestion.

4. **Add or update tests** for any behavior change. `core/engine`, `core/runtime/rhai`, and `core/dom` are the reference
   for what "well-tested" looks like in this codebase.

5. **New architectural decision?** Add an ADR under `docs/adr/` (MADR format) plus a row in `docs/adr/README.md`,
   **in the same PR as the code it justifies** — not as a follow-up after review asks for it.

6. **Run the gate before pushing:**

    ```bash
    just gate
    ```

   This mirrors CI: `fmt-check`, `clippy -D warnings`, `check`, `test`, `cargo-deny`, coverage, `arch-lint`, and the
   `no-engine` dependency check. Git hooks (via [Lefthook](https://lefthook.dev/)) run a subset of this automatically on
   `pre-commit`/`pre-push`, but a full `just gate` catches everything before you open the PR.

7. **Format Markdown.** Any `.md` file you touch should pass `rumdl fmt` (config in `.rumdl.toml`) — the pre-commit hook
   rewrites it for you if you forget, but running it yourself avoids an extra commit.

## Commit Messages

Use a short, imperative summary line (`feat(dom): add first-class AttributeMap`, `fix(css): …`, `docs(adr): …`).
Conventional-commit-style prefixes (`feat`, `fix`, `docs`, `refactor`, `test`, `chore`) matching the crate/area touched
are preferred, matching the existing commit history.

## Opening a Pull Request

- Fill out the [pull request template](.github/pull_request_template.md) — in particular, the test plan.
- Keep the PR focused on one logical change. Large or stacked work should be split into reviewable increments.
- Make sure `just gate` is green and CI passes before requesting review.
- Link the issue the PR closes, if any (`Closes #123`).

## Reporting Bugs and Requesting Features

Use the issue templates under **New Issue** — a [bug report](.github/ISSUE_TEMPLATE/bug_report.yml) or a
[feature request](.github/ISSUE_TEMPLATE/feature_request.yml). See [`SECURITY.md`](SECURITY.md) instead if you're
reporting a security vulnerability — do not open a public issue for those.

Issues carry one `type:*` label (the issue forms set it), an `area:*` label that follows the crate map, a release
milestone (`vX.Y.Z`) and, unless they are an `epic`, an open parent issue. The maintainers assign the rest while
triaging. Labels, milestones and merge settings are declared in [`tools.toml`](tools.toml) and checked by
[`jaoppb/tools`](https://github.com/jaoppb/tools), which the flake shell provides:

```bash
just issues-check   # audit issues: type, area, milestone, parent, pending label migrations (needs `gh auth login`)
just repo-check     # compare GitHub labels, milestones and merge settings with tools.toml
just config-check   # tools.toml parses and needs no schema migration
```

To add a label or milestone, edit `tools.toml`; `just repo-sync` applies it and is run by a maintainer.

Issue state is also shown on the [Alloy GitHub Project](https://github.com/users/jaoppb/projects/8). Move your card to
In progress when you start; the `[workflow]` rules in `tools.toml` derive the rest (Backlog, Ready, In review once a
pull request links, Done) from labels, milestone, blocked-by links, parent and linked pull requests, and epic cards stay
where they are. A maintainer runs `just project-check` (writes nothing) and `just project-sync` locally.

## Code of Conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md). By participating, you're expected to uphold it.

## Questions

See [`SUPPORT.md`](SUPPORT.md) for where to ask for help.
