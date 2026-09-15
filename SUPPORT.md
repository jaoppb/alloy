# Support

This document explains where to get help with Alloy.

## Documentation First

- [`README.md`](README.md) — what Alloy is, key features, architecture overview.
- [`CLAUDE.md`](CLAUDE.md) — the canonical architecture, coding-standard, and workflow reference (used by both human
  contributors and AI coding agents).
- [`docs/adr/`](docs/adr) — Architecture Decision Records explaining _why_ the system is built the way it is.
- [`docs/requirements/`](docs/requirements) — the PRDs each subsystem is built against.
- [`docs/reports/`](docs/reports) — implementation reports for shipped milestones.

Most "why does X work like this" and "how do I do Y" questions are already answered in one of the above.

## Asking a Question

If the docs don't answer your question:

1. Search [existing issues](https://github.com/jaoppb/alloy/issues?q=is%3Aissue) — someone may have asked already.
2. Open a [new issue](https://github.com/jaoppb/alloy/issues/new/choose) using the **feature request** template if your
   question is really "should Alloy do X", or file it as a regular issue for anything else. There is no separate
   discussion forum yet — issues are the support channel.

## Reporting a Bug

Use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.yml). Include the command you ran, what you expected,
what happened instead, and your `rustc`/`cargo` version (`rust-toolchain.toml` pins the version Alloy expects —
mismatches are a common source of confusing build errors).

## Reporting a Security Vulnerability

**Do not open a public issue.** See [`SECURITY.md`](SECURITY.md) for the private reporting process.

## Contributing

If you'd like to fix something yourself, see [`CONTRIBUTING.md`](CONTRIBUTING.md) for the development setup and
workflow.
