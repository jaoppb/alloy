# Pull Request

<!--
    Thanks for contributing to Alloy! Please fill out this template — it mirrors what reviewers check for, so a
    complete PR description is usually the fastest way to get through review. See CONTRIBUTING.md for the full
    workflow and CLAUDE.md for architecture/coding-standard rules.
-->

## Summary

<!-- What does this PR do, and why? Link the issue it closes, if any. -->

Closes #

## Type of Change

- [ ] Bug fix
- [ ] New feature
- [ ] Refactor (no behavior change)
- [ ] Documentation
- [ ] Tooling / CI
- [ ] Architecture Decision Record (ADR)

## Checklist

- [ ] I read [`CLAUDE.md`](../CLAUDE.md) and followed the crate layering (`domain/` → `application/` →
      `infrastructure/`) and coding standard (Clean Code + Object Calisthenics) for any code I touched.
- [ ] `just gate` passes locally (fmt, clippy `-D warnings`, check, test, `cargo-deny`, coverage, `arch-lint`,
      `no-engine`).
- [ ] I added or updated tests for any behavior change.
- [ ] If this introduces a new architectural or tooling decision, I added an ADR under `docs/adr/` (MADR format) plus a
      row in `docs/adr/README.md`, in this same PR.
- [ ] If this changes a documented invariant (a PRD or ADR claim), I checked it still holds — or opened a tracked,
      explicit exception.
- [ ] `.md` files I touched are formatted with `pnpm format:md`.

## Test Plan

<!-- How did you verify this works? Commands run, manual testing steps, etc. -->

- [ ] `just gate`
- [ ]

## Additional Context

<!-- Anything a reviewer needs to know: design trade-offs, follow-up work left for a later PR, screenshots, etc. -->
