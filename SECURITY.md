# Security Policy

## Supported Versions

Alloy is pre-1.0 (`0.x`) and under active, rapid development. There are no maintained release branches yet — only the
latest commit on `main` receives security fixes.

| Version                 | Supported          |
| ----------------------- | ------------------ |
| `main` (latest)         | :white_check_mark: |
| Any tagged/older commit | :x:                |

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Report vulnerabilities privately using
[GitHub Security Advisories](https://github.com/jaoppb/alloy/security/advisories/new) for this repository. This opens a
private channel with the maintainers where the report, discussion, and fix can happen before any public disclosure.

If you cannot use GitHub Security Advisories for any reason, contact [@jaoppb](https://github.com/jaoppb) directly
through GitHub.

When reporting, please include:

- A description of the vulnerability and its potential impact.
- Steps to reproduce, or a minimal script/`.rhai` sample if the issue involves script execution.
- The commit hash or version affected.
- Whether the issue is in the trusted Rhai muscle layer (`core/engine` + `core/runtime/rhai`), the untrusted web-content
  layer (`core/js`), or elsewhere — this changes the severity model (see below).

You can expect an initial response within a few days. We'll keep you updated as the issue is triaged, fixed, and
disclosed.

## Security Model

Alloy draws a hard line between two script execution contexts (see `CLAUDE.md` → Architecture):

- **`core/engine` + `core/runtime/rhai`** run **trusted** browser-customization scripts. They are still sandboxed: every
  execution context carries an explicit capability bitflag set (`DOM_READ`, `DOM_MUTATE`, `NETWORK_FETCH`,
  `GRAPHICS_DRAW`, `WINDOW_MANAGE`, …), over-reach returns a typed `PermissionDenied` error, and script panics/errors
  are trapped (`catch_unwind`) so a misbehaving script can never abort the host process (see ADR-0004).
- **`core/js`** is the (stubbed, as of this writing) engine intended to run **untrusted** web-page `<script>` content,
  and must not be conflated with the trusted muscle engine above.

Cross-cutting invariants relevant to security:

- `#![forbid(unsafe_code)]` on every crate in the workspace.
- `cargo-deny` (`deny.toml`) audits the dependency graph for known advisories, and is a blocking CI gate.
- `core/engine` depends on nothing but `bitflags`, enforced by the CI `no-engine` job — the port abstraction itself has
  no attack surface to audit beyond that one crate.

A vulnerability that lets an untrusted script (or a trusted script without the right capability) escape its sandbox,
read/write memory it shouldn't, or crash the host process is a valid security report even before `core/js` and the rest
of the rendering pipeline are implemented.
