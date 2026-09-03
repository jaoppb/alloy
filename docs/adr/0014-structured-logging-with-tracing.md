# ADR-0014: Structured Logging with `tracing`

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-08-30

---

## Context and Problem Statement

v0.1 shipped with `println!` / `eprintln!` scattered through the `alloy` binary. A browser engine with a hot-reload
loop, a capability sandbox, a network stack and a render pipeline needs diagnostics that carry structured context
(subsystem, script name, capability, timing), that can be filtered per module without a recompile, and that can be
emitted as machine-readable JSON for later tooling. Raw `print` macros give none of that and cannot be turned off.

We need one logging facade for the whole workspace, chosen deliberately and recorded.

---

## Decision Drivers

- One facade, used the same way in every crate; libraries emit, the binary configures.
- Level filtering from the environment, no recompile (`RUST_LOG` / a project-specific variable).
- A human-readable format by default and a structured (JSON) format on demand.
- Spans, not just events — the hot-reload / eval / request paths are nested scopes.
- Minimal dependency weight; no `unsafe` on the hot path (N-02).

---

## Considered Options

- **Option 1**: **`tracing` + `tracing-subscriber`** — span-aware, `EnvFilter` for runtime filtering, `fmt` layer for
  pretty output and a `json` layer for structured output, de-facto standard in the async Rust ecosystem.
- **Option 2**: `log` + `env_logger` — event-only (no spans), simplest, but a dead end the moment nesting matters.
- **Option 3**: `slog` — structured and fast, but a smaller ecosystem and a heavier API; declining adoption.
- **Option 4**: keep `println!` / `eprintln!` — zero deps, zero capability.

---

## Decision Outcome

Chosen option: **Option 1 (`tracing` + `tracing-subscriber`)**.

- **Libraries** (`core/*`, `devtools`, `extension`) depend only on `tracing` and only ever call its macros (`error!` /
  `warn!` / `info!` / `debug!` / `trace!`, `#[instrument]`). They never touch a subscriber.
- **`alloy`** (and any future binary) owns `tracing-subscriber` and installs exactly one global subscriber at startup:
    - `EnvFilter` seeded from `ALLOY_LOG` (falling back to `RUST_LOG`), **default `info`**. Never a hardcoded `Level` —
      enforced by `arch-lint` rule `tracing-env-init` (AL007).
    - **Default format: `fmt` pretty** (human-readable, to stderr).
    - **`ALLOY_LOG_FORMAT=json`** switches to the `json` layer for structured output.
- Direct `println!` / `eprintln!` are not used for diagnostics anywhere in the workspace. The `alloy` binary's own
  diagnostics and its script-result line go through `tracing`; `clap` writes `--help` / `--version` through its own
  machinery, which is out of scope for this rule.

### Consequences

- **Positive**:
    - Per-module runtime filtering; JSON output available without a rebuild.
    - Spans model the hot-reload / eval / request scopes the roadmap needs.
    - `arch-lint` (`require-tracing`, `tracing-env-init`) mechanically enforces the facade and the env-driven init.
- **Negative**:
    - `tracing-subscriber` pulls `regex` / `sharded-slab` into the binary (not the libraries).
    - Every new binary must remember to install a subscriber or its libraries' events go nowhere.
