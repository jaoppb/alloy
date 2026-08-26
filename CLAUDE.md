# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Current State

The workspace is a **bootstrap skeleton**: all 11 crates exist with `Cargo.toml` + a stub `src/lib.rs` (the default
`add()` / `it_works()` template). Only `rhai-runtime` declares a dependency (`engine`). The architecture below is
specified in `docs/` and not yet implemented — treat `docs/adr/` and `docs/requirements/` as the authoritative design
contract when writing new code, and expect to create the `domain/` / `application/` / `infrastructure/` module trees
yourself.

## Commands

Tooling is split: Cargo for Rust, pnpm for Markdown quality gates.

```bash
pnpm check                                  # full gate: prettier check + markdownlint + cargo fmt --check + clippy
cargo test --workspace                      # all tests (also `pnpm test`)
cargo test -p dom                            # one crate
cargo test -p dom -- node::tests::name       # one test
cargo check --workspace --all-targets
pnpm lint:rust                               # clippy --workspace --all-targets --all-features -D warnings
pnpm format:rust                             # cargo fmt --all
pnpm format:md / pnpm lint:md                # prettier --write / markdownlint-cli2
```

Git hooks are managed by **Lefthook** (`lefthook.yml`): pre-commit runs `cargo fmt` + clippy and prettier + markdownlint
(auto-staging fixes); pre-push runs `cargo test --workspace` and `cargo check --workspace --all-targets`. Clippy
warnings are errors — never leave one behind.

Markdown formatting is enforced with **tabs, tab width 4, print width 120, `proseWrap: always`** (`.prettierrc.json`).
Run `pnpm format:md` after editing any `.md` file or the commit hook will rewrite it.

## Architecture

**Skeleton and Muscle** (ADR-0003): Rust owns all data structures, memory, and OS I/O (the Skeleton). Scripts own
behavior — event routing, policy, pipeline composition (the Muscle). When adding a feature, decide which side it belongs
to; hardcoding user-facing policy into Rust violates the core pattern.

### Crate map

| Path                | Package        | Responsibility                                                                                                                                                                       |
| ------------------- | -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `core/engine`       | `engine`       | `RuntimeEngine` / `ExecutionContext` traits, `EngineValue`, capability bitflags. Pure type/macro deps (`thiserror`, `bitflags`), I/O watchers strictly isolated in `infrastructure/` |
| `core/runtime/rhai` | `rhai-runtime` | Concrete Rhai backend implementing `engine`'s traits                                                                                                                                 |
| `core/js`           | `js`           | Web-content ECMAScript runtime (untrusted page `<script>`) — distinct from the Rhai muscle engine                                                                                    |
| `core/dom`          | `dom`          | Node hierarchy, elements, mutations                                                                                                                                                  |
| `core/html`         | `html`         | HTML5 tokenization & tree construction                                                                                                                                               |
| `core/css`          | `css`          | Parser, rule sets, cascade                                                                                                                                                           |
| `core/graphics`     | `graphics`     | `DisplayList`, Vulkan/OpenGL/software renderers                                                                                                                                      |
| `core/window`       | `window`       | Window creation, event loop, surface binding                                                                                                                                         |
| `core/network`      | `network`      | Sockets, DNS, HTTP/1.1 & HTTP/2, cache                                                                                                                                               |
| `devtools`          | `devtools`     | Debug protocol, inspector, hot-reload orchestration                                                                                                                                  |
| `extension`         | `extension`    | WebExtensions bridge                                                                                                                                                                 |

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
