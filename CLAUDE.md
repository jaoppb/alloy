# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Current State

**v0.1 ("O engine vive") is delivered — F0 + F1 + F2.** Done by the _solid_ path (ADR-0011 Replaceable Port Contract),
not the "PRD-002 verbatim / `core/engine → rhai`" path the v0.1 report originally proposed; see the amendments in
`docs/reports/IMPLEMENTACAO-DETALHADA-V0-1.md`. The PR #4 review response then added: strict
`[workspace.lints.clippy]` + `clippy.toml`, a `justfile` (replacing the Makefile), `arch-lint`, `tracing` in `alloy`
(ADR-0014), `thiserror` outside `core/engine` (ADR-0015), and the object-calisthenics VOs `FunctionName` /
`VariableName` / `Line` / `Column` (port schema **v2** — every name on the port is a validated newtype).

- **Foundation**: `rust-toolchain.toml` (pins 1.97.1), versioned `Cargo.lock`, `[workspace.package]` +
  `[workspace.dependencies]`, `deny.toml`, `.github/workflows/ci.yml`, `#![forbid(unsafe_code)]` on every crate.
- **`core/engine`**: `domain/` (`EngineValue` / `ValueKind` — `#[non_exhaustive]`, `EngineError` — one enum,
  `Capability`/`CapabilitySet`, `ExecutionLimits`, `SourceLocation`), `application/ports.rs` (`RuntimeEngine` /
  `ExecutionContext` — PRD-002 with two documented deviations: no associated `type Error`; `EngineType` instead of
  `rhai::CustomType`; PRD-002 generics kept as provided sugar over an object-safe core), conversion / `EngineFunction` /
  `EngineType` traits, public `engine::conformance` suite, `engine::PORT_SCHEMA_VERSION` (= 2, ADR-0011 items 3/7; see
  PRD-002 §4.2). Depends only on `bitflags` — enforced by the CI `no-engine` job. `MockEngine` reference adapter in
  `tests/` closes **C-01, C-05**. ADR-0011 contract state: `docs/architecture/runtime-engine-port-contract.md` (items
  1,3,4,5,6 ✅; item 2 `dyn RuntimeEngine` companion deferred to v0.2/ADR-0013). Verified locally (`just gate`):
  `cargo deny check` green, `cargo llvm-cov -p engine` ≈ 95% lines.
- **`core/runtime/rhai`**: `RhaiEngine` / `RhaiContext` / `RhaiCompiledScript(Arc<rhai::AST>)`;
  `EngineValue ⇄ rhai::Dynamic` marshaling; `set_max_operations`/`set_max_call_levels`/`set_max_expr_depths` + a
  wall-clock `on_progress` guard → `ExecutionLimitExceeded` (**C-04**); `catch_unwind` → `ScriptPanic` (mechanism of
  C-09); `RhaiContext::register_custom_type::<T: EngineType + rhai::CustomType>` bridge. **The only crate that names a
  `rhai` type.** `tests/` run the shared conformance suite + `FixtureNode` (**C-02**).
- **`alloy`** binary: `cargo run -p alloy` prints help and exits 0; `alloy --script <path>` runs a `.rhai` file under
  the sandbox and prints the result. `clap` derive for args, typed `AlloyError` (`thiserror`) for failures. Explicit
  workspace member (the `core/runtime/*` glob is untouched). Example: `scripts/hello.rhai`.

Still **stubs** (doc-comment + `#![forbid(unsafe_code)]` only, no items yet): `core/dom`, `core/html`, `core/css`,
`core/graphics`, `core/window`, `core/network`, `core/js`, `devtools`, `extension`. Open criteria: C-03 (v0.2 I1 — real
`DomNode`), C-06 … C-18. Follow the `domain/` / `application/` / `infrastructure/` layering that `core/engine` and
`core/runtime/rhai` now demonstrate; `docs/adr/` + `docs/requirements/` remain the authoritative contract.

## Commands

Tooling is split: Cargo for Rust, pnpm for Markdown quality gates. A root `justfile` wraps both — `just` lists every
recipe.

```bash
just gate                                    # full local gate (fmt-check + clippy + check + test + deny + coverage + arch + no-engine) — mirrors CI
just setup                                   # one-time: pnpm deps, rust components, cargo-deny, cargo-llvm-cov, arch-lint, git hooks
just test engine "-- name"                   # scoped test run
just run --script scripts/hello.rhai         # run the alloy binary
just deny | just coverage | just no-engine   # individual CI gates

# equivalent raw commands:
pnpm check                                  # prettier check + markdownlint + cargo fmt --check + clippy
cargo test --workspace                      # all tests (also `pnpm test`)
cargo test -p dom -- node::tests::name       # one test
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all
```

Git hooks are managed by **Lefthook** (`lefthook.yml`): pre-commit runs `cargo fmt`, clippy, `arch-lint`, prettier and
markdownlint (auto-staging fixes); pre-push runs `cargo test --workspace` and `cargo check --workspace --all-targets`.
Clippy warnings are errors — never leave one behind. The strict lint set lives in `[workspace.lints.clippy]` (root
`Cargo.toml`) plus `clippy.toml`; `arch-lint.toml` adds the `tracing` / no-`unwrap` code-pattern rules.

Markdown formatting is enforced with **tabs, tab width 4, print width 120, `proseWrap: always`** (`.prettierrc.json`).
Run `pnpm format:md` after editing any `.md` file or the commit hook will rewrite it.

## Architecture

**Skeleton and Muscle** (ADR-0003): Rust owns all data structures, memory, and OS I/O (the Skeleton). Scripts own
behavior — event routing, policy, pipeline composition (the Muscle). When adding a feature, decide which side it belongs
to; hardcoding user-facing policy into Rust violates the core pattern.

### Crate map

| Path                | Package        | Responsibility                                                                                                            |
| ------------------- | -------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `core/engine`       | `engine`       | `RuntimeEngine` / `ExecutionContext` traits, `EngineValue`, capability bitflags. **Zero dependencies — pure abstraction** |
| `core/runtime/rhai` | `rhai-runtime` | Concrete Rhai backend implementing `engine`'s traits                                                                      |
| `core/js`           | `js`           | Web-content ECMAScript runtime (untrusted page `<script>`) — distinct from the Rhai muscle engine                         |
| `core/dom`          | `dom`          | Node hierarchy, elements, mutations                                                                                       |
| `core/html`         | `html`         | HTML5 tokenization & tree construction                                                                                    |
| `core/css`          | `css`          | Parser, rule sets, cascade                                                                                                |
| `core/graphics`     | `graphics`     | `DisplayList`, Vulkan/OpenGL/software renderers                                                                           |
| `core/window`       | `window`       | Window creation, event loop, surface binding                                                                              |
| `core/network`      | `network`      | Sockets, DNS, HTTP/1.1 & HTTP/2, cache                                                                                    |
| `devtools`          | `devtools`     | Debug protocol, inspector, hot-reload orchestration                                                                       |
| `extension`         | `extension`    | WebExtensions bridge                                                                                                      |

Package names are bare (no `alloy-` prefix), so deps read `dom = { path = "../dom" }` (`core/runtime/rhai` needs
`../../engine`). Workspace members are listed **explicitly** in the root `Cargo.toml` plus a `core/runtime/*` glob — a
bare `core/*` glob also matches the manifest-less `core/runtime/` directory and breaks every Cargo command. Add new
script backends under `core/runtime/`; add anything else as an explicit member.

Domain crates must **never** depend on `rhai` or any interpreter directly — they go through `engine` traits (ADR-0002).
Two script boundaries exist and must not be conflated: `core/js` runs untrusted web content; `core/engine` +
`core/runtime/rhai` run trusted browser-customization scripts.

### Per-crate layer structure (ADR-0010)

```text
src/lib.rs            # public facade / exported ports
src/domain/           # entities, newtype value objects, typed errors — zero deps, zero I/O
src/application/      # pipeline orchestrators, domain services, ports (traits)
src/infrastructure/   # adapters implementing those ports (vulkano, sockets, rhai bindings)
```

Dependencies point inward only: `domain` → nothing, `application` → `domain`, `infrastructure` → both. UI-ish crates
(`devtools`, `extension`) use Feature-Sliced Design (`app/`, `features/`, `widgets/`, `shared/`) instead.

Crates communicate by passing immutable aggregates down a pipeline:
`HtmlStream → DomTree → StyledTree → LayoutBoxTree → DisplayList → RenderBackend`. Cross-crate conversions go through
explicit mapping functions/DTOs — no type leaking.

### Cross-cutting invariants

- **Capability sandboxing** (ADR-0004): every `ExecutionContext` is created with an explicit capability bitflag set
  (`DOM_READ`, `DOM_MUTATE`, `NETWORK_FETCH`, `GRAPHICS_DRAW`, `WINDOW_MANAGE`, …). Over-reach returns
  `EngineError::PermissionDenied`. Script panics/errors are trapped and must never abort the host process.
- **Hot reload** (ADR-0005): scripts are stateless; reload is an atomic `Arc<CompiledAST>` swap. A failed compile keeps
  the previous AST alive and reports to DevTools — never leaves a half-loaded state. Rust keeps all durable state.
- **Graphics tiers** (ADR-0009): Vulkan (`vulkano`) → OpenGL (`glow`/`glutin`) → CPU software rasterizer for headless
  CI. Layout code emits a declarative `DisplayList` and stays GPU-API agnostic.

### Object Calisthenics (ADR-0010, enforced)

No naked primitives in domain models — use newtypes (`NodeId(u32)`, `TagName(String)`, `Px(f32)`, `Color(u32)`).
First-class collections (`Children`, `RuleSet`, `HeaderMap`). No `else` (early return / `match` / `if let`), one
indentation level per function, one dot per line, no abbreviated names (`element_identifier`, not `el_id`), no public
mutable fields — mutate through invariant-validating methods.

## SPDD Workflow

Feature work follows Structured Prompt-Driven Development (ADR-0007): `/spdd-analysis` (→ `spdd/analysis/`) →
`/spdd-reasons-canvas` (→ `spdd/prompt/`, the 7-stage REASONS canvas) → `/spdd-generate` → `/spdd-sync`. The skill
definitions live in `.agents/skills/` (`.cursor` is a symlink to `.agents`); Claude Code does not auto-discover that
directory, so read the relevant `SKILL.md` before following the flow. `docs/requirements/PRD-*.md` are the authoritative
inputs, `docs/adr/*.md` the constraints.

New architectural decisions get an ADR in `docs/adr/` (MADR format) plus a row in `docs/adr/README.md`.
