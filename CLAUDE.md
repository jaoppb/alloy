# CLAUDE.md

This file provides guidance to AI coding agents (Claude Code, Gemini CLI — see `GEMINI.md`, a pointer to this file) when
working with code in this repository. Do not fork guidance into `GEMINI.md`: recurring-mistake documentation only works
if every agent reads the same corrections, so this file is the single source and `GEMINI.md` stays a pointer, never a
copy.

## Current State

**v0.1 ("O engine vive") is delivered — F0 + F1 + F2.** Done by the _solid_ path (ADR-0011 Replaceable Port Contract),
not the "PRD-002 verbatim / `core/engine → rhai`" path the v0.1 report originally proposed; see the amendments in
`docs/reports/IMPLEMENTACAO-DETALHADA-V0-1.md`. The PR #4 review response then added: strict
`[workspace.lints.clippy]` + `clippy.toml`, a `justfile` (replacing the Makefile), `arch-lint`, `tracing` in `alloy`
(ADR-0014), `thiserror` outside `core/engine` (ADR-0015), and the object-calisthenics VOs `FunctionName` /
`VariableName` / `Line` / `Column` (port schema **v2** — every name on the port is a validated newtype).

**v0.2 ("DOM scriptável e contido") delivered — F3 + I1 + F6.** Closes C-03, C-06, C-07, C-08, C-09. SPDD canvases:
`spdd/{analysis,prompt}/202608300315-*-f3.md` and `…320-*-f6-i1.md`. **Both v0.1 and v0.2 are merged to `main`**
(`75dc34b`, `0a60036`; PRs #4, #5, #6 all merged).

- **F6 — `core/engine`**: `application/dyn_bridge.rs` (ADR-0013) — the object-safe `dyn` companion: `DynRuntimeEngine` /
  `DynExecutionContext` / `DynCompiledScript` + free `eval_typed`, all blanket-impl'd, no F1 signature change.
  `engine::conformance::run_dyn_suite` — `MockEngine` and `RhaiEngine` pass it alongside `run_core_suite`. Contract
  record item 2 → ✅.
- **F6 — `core/runtime/rhai`**: `infrastructure/context.rs:80` —
  `RhaiContext::register_guarded_binding(name, arity, required, handler)` is the single capability chokepoint
  (`CapabilitySet` captured by value; guard is `and` of bits + branch); `guarded_bindings()` (`context.rs:100`) is the
  C-06 sweep; `infrastructure/sandbox.rs` holds the `GuardedBinding` table type and `install_guarded_table`. v0.2 ships
  **no** production top-level guarded binding (all DOM access is `NodeHandle` methods, which self-guard) — the mechanism
  is tested and ready. `infrastructure/fallback.rs` — `run_with_fallback` (primary → stderr diagnostic +
  `SourceLocation` → embedded `scripts/default_dom.rhai` on a **clean** tree → Rust `minimal_document()`),
  `PanicHookGuard` scoped around each eval. Tests: `dyn_conformance`, `isolation` (C-08: scope / capability /
  trapped-panic isolation), `fault_injection` (C-09: panic in a guarded binding trapped for every capability;
  `run_with_fallback` recovers from every error class incl. the default script failing). CI job `fault-injection`
  (`--test-threads=1`) is **blocking**.
- **F6 — `alloy`**: `--script` runs through `run_with_fallback` — a failing script writes a diagnostic, the fallback DOM
  is printed, and the process exits 0.

- **Foundation**: `rust-toolchain.toml` (pins 1.97.1), versioned `Cargo.lock`, `[workspace.package]` +
  `[workspace.dependencies]`, `deny.toml`, `.github/workflows/ci.yml`, `#![forbid(unsafe_code)]` on every crate.
- **`core/engine`**: `domain/` (`EngineValue` / `ValueKind` — `#[non_exhaustive]`, `EngineError` — one enum,
  `Capability`/`CapabilitySet`, `ExecutionLimits`, `SourceLocation`), `application/ports.rs` (`RuntimeEngine` /
  `ExecutionContext` — PRD-002 with two documented deviations: no associated `type Error`; `EngineType` instead of
  `rhai::CustomType`; PRD-002 generics kept as provided sugar over an object-safe core), `EngineType` traits, public
  `engine::conformance` suite (`run_core_suite` + `run_dyn_suite`), `engine::PORT_SCHEMA_VERSION` (= 2 since v0.2 added
  `EngineError::Dom` and v0.1 added VOs; ADR-0011 items 3/7). Depends only on `bitflags` — enforced by the CI
  `no-engine` job. `MockEngine` reference adapter in `tests/` closes **C-01, C-05**. ADR-0011 contract state:
  `docs/architecture/runtime-engine-port-contract.md` — **all seven items ✅** (item 2's `dyn` companion landed in v0.2
  F6, ADR-0013). Verified locally (`just gate`): `cargo deny check` green, `cargo llvm-cov -p engine` ≈ 95% lines.
- **`core/runtime/rhai`**: `RhaiEngine` / `RhaiContext` / `RhaiCompiledScript(Arc<rhai::AST>)`;
  `EngineValue ⇄ rhai::Dynamic` marshaling; `set_max_operations`/`set_max_call_levels`/`set_max_expr_depths` + a
  wall-clock `on_progress` guard → `ExecutionLimitExceeded` (**C-04**); `catch_unwind` → `ScriptPanic` (mechanism of
  C-09); `RhaiContext::register_custom_type::<T: EngineType + rhai::CustomType>` bridge. **The only crate that names a
  `rhai` type.** (`PORT_SCHEMA_VERSION` lives only in `core/engine/src/lib.rs:65`; this crate consumes it.) `tests/` run
  the shared conformance suite + `FixtureNode` (**C-02**) + `scriptable_dom` (**C-03** and the I1 slice of C-06/C-07).
- **`core/dom`** (v0.2 F3): pure domain crate, **one dependency** (`thiserror`, ADR-0015), `#![forbid(unsafe_code)]`,
  all nine Object Calisthenics rules (no exception). `domain/` — arena `DomTree` (`Vec<Slot>` by `NodeId(u32)`; removal
  → `Tombstone`, index never reused) enforcing the five invariants of report §2.2 through its methods only; value
  objects (`TagName` / `AttributeName` validated + lowercased, `TextContent`, `CommentContent`, `AttributeValue`);
  first-class `Children` / `AttributeMap` (insertion order); one `#[non_exhaustive]` `DomError` (9 variants; never names
  `EngineError`); non-recursive `Descendants` / `Ancestors`. `application/serialize.rs` — deterministic `serialize_html`
  (escapes `&<>`, void elements). 15 tests.
- **`core/runtime/rhai` I1**: `infrastructure/dom_bindings.rs` — `NodeHandle` (`EngineType` + `rhai::CustomType`, script
  name `Node`) holding `Arc<Mutex<DomTree>>` + `NodeId` + a baked-in `CapabilitySet`; each method self-guards
  (`DOM_READ` reads, `DOM_MUTATE` mutators) and maps `DomError` → `EngineError::Dom`. `NODE_HANDLE_BINDINGS` is the
  guarded-method manifest. `RhaiContext::bind_dom(Arc<Mutex<DomTree>>)` registers `Node` and the global `document`
  handle. `native::to_eval_error` now boxes the `EngineError` in `EvalAltResult::ErrorSystem` and `error_map` downcasts
  it back, so a `PermissionDenied` / `Dom` raised inside a binding round-trips to the host as that exact variant (C-07).
  **Deviation from report §2.5/2.7**: `Arc<Mutex<_>>` not `Rc<RefCell<_>>`, `RhaiContext` stays `Send + Sync` — the
  `rhai` `sync` feature (required for `RuntimeEngine: Send + Sync`) forces `CustomType: Send + Sync`.
- **`alloy`** binary: `cargo run -p alloy` prints help and exits 0; `alloy --script <path>` runs a `.rhai` file under
  the sandbox with a bound DOM (`DOM_READ | DOM_MUTATE`), logs return value via `tracing`, prints serialized HTML, and
  falls back safely on script error. `clap` derive for args, typed `AlloyError` (`thiserror`) for failures, `tracing`
  (ADR-0014) for structured diagnostics. Examples: `scripts/hello.rhai`, `scripts/hello_dom.rhai`.

Still **stubs** (8 lines: a doc comment and `#![forbid(unsafe_code)]` — no functions at all): `core/html`, `core/css`,
`core/graphics`, `core/window`, `core/network`, `core/js`, `devtools`, `extension`. Open criteria: C-10 … C-18 (v0.3+).
Follow the `domain/` / `application/` / `infrastructure/` layering that `core/engine`, `core/runtime/rhai`, and
`core/dom` now demonstrate; `docs/adr/` + `docs/requirements/` remain the authoritative contract.

## Recurring Review Feedback — Read Before Opening a PR

Mined from every review comment on PRs #1–#11 (`gh api repos/jaoppb/alloy/pulls/<N>/{reviews,comments}`). These patterns
were flagged **more than once, in more than one PR** — each is a mistake an agent re-introduced after it had already
been corrected somewhere else in the codebase. This section exists so that stops happening; check it before every push,
and especially before starting a fresh stacked branch off `main` (PRs #10–#20 are parallel stacked branches — a decision
recorded here or in `docs/adr/` may already postdate the commit your branch forked from).

1. **Naked primitive strings for domain identifiers.** By far the most repeated finding (PR #2, #4, #5, #10, #11):
   function/variable names, tag names, attribute names, HTTP media types — all initially shipped as raw `&str`/`String`
   instead of a validated newtype or enum, then had to be redone as `FunctionName`, `VariableName`, `TagName`,
   `AttributeName`, `MediaType`, etc. This is not a style nit — it is the Object Calisthenics rule above ("no naked
   primitives") and CI/review will catch it every time. When a value has a valid/invalid distinction (an identifier
   format, a closed vocabulary like an HTML tag or an HTTP method) it is a newtype or enum from the first commit, never
   a bare string with call sites doing `&str` comparisons.
2. **`arch-lint.toml` reinvented per-branch instead of extended.** PR #4, #5, #10, #11 each got "replace this with a
   rule at `arch-lint.toml`" — a stacked branch re-adding an ad hoc `cargo tree | grep` / manual dependency check
   instead of adding a `[[deny-scope-dep]]` / `[[restrict-use]]` rule to the existing config. Before writing any new
   architectural check (dependency direction, layer boundary), read `arch-lint.toml` first — the mechanism almost
   certainly already exists; extend it, don't shadow it with a bespoke script.
3. **`println!`/`eprintln!` creeping back in.** PR #2, #4, #5: raw print macros instead of `tracing` (ADR-0014). This is
   settled — use `tracing::info!`/`warn!`/`error!` with `EnvFilter` from the first line of new code, in every crate that
   logs, including a fresh stacked branch that forked before ADR-0014 landed.
4. **Manual error enums instead of `thiserror`.** PR #2, #4, #5: hand-rolled `impl Display`/`Error` where `thiserror`
   (ADR-0015) applies. `core/engine` is the one deliberate exception (hand-written `Display`/`Error`, see Object
   Calisthenics note above) — everywhere else, derive with `thiserror` from the start.
5. **Wrong collection for keyed/lookup data.** PR #5, #10: `Vec<(K, V)>` used where a `BTreeMap`/`HashMap`-backed
   first-class collection was needed for correctness (ordering, dedup) or `O(1)`/`O(log n)` lookup — e.g.
   `AttributeMap`, a guarded-binding table. Pick the collection for the access pattern the type actually needs, not the
   first one that compiles.
6. **Domain logic implemented in `application` (or leaking into `infrastructure`).** PR #2, #5: CSS parsing, DOM
   serialization detail (`is_void`, entity escaping) initially placed in `application/` when the decision belongs in
   `domain/` types themselves (e.g. `TagName::is_void()`), and `infrastructure` coupling a concrete crate (`notify`)
   directly into `application` instead of behind a port. Re-check layer placement (per the layering table below) before
   the first commit of a new type, not after review.
7. **GitHub Actions pinned to stale versions.** PR #2, #4: `actions/checkout`, `setup-node`, `cache`, etc. left on an
   old major. Check the action's current stable major before pinning a version in `.github/workflows/ci.yml`, and keep
   `node-version` on the current LTS/latest used elsewhere in the workflow.
8. **A documented invariant silently goes false.** PR #4: introducing `rhai` made `PRD-001:97` ("zero unsafe memory
   operations exposed to script runtimes") false by vacuity-loss — `main` had no script engine, so the requirement was
   trivially true until this PR made it checkable and failing. Nobody caught it until a manual audit. When a PR adds a
   new dependency or crosses a boundary a PRD/ADR makes a claim about, grep the claim against the new code _before_
   opening the PR and either satisfy it or open a tracked, explicit exception — never let review discover it.
9. **A missing ADR for a tooling/architecture decision.** PR #2, #4: `thiserror` adoption, the `justfile` / `arch-lint`
   swap, `tracing` adoption were all implemented before an ADR existed to justify them; reviewers asked for the ADR
   after the fact. If the change is the kind of decision `docs/adr/README.md` tracks, write the ADR in the same PR as
   the code, not as a follow-up.

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
cargo test -p dom --test tree_invariants     # one integration-test file
cargo test -p engine mock_engine             # tests matching a name
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

### Clean Code + Object Calisthenics (ADR-0010, enforced)

Write to _Clean Code_ (Robert C. Martin) as the baseline; Object Calisthenics is the strict subset that CI and review
check mechanically. Both apply to every hand-written `core/*` line — `core/engine`, `core/runtime/rhai`, and `core/dom`
are the reference.

**Clean Code baseline:**

- **Intention-revealing names.** Full words from the Ubiquitous Language (`attribute_name`, not `attr`); no encodings,
  no noise words. A name that needs a comment to be understood is the wrong name.
- **Small functions, one job, one level of abstraction.** A function either orchestrates or does detail work, never
  both. Extract a private helper before a function grows a second reason to change or a second indentation level.
- **Command–Query Separation.** A method either changes state and returns `()` (`DomTree::append_child`) or answers a
  question and mutates nothing (`DomTree::tag`) — never both.
- **No boolean/flag parameters.** Split into two named methods or take a small enum (`Attachment::End` /
  `Attachment::Before`).
- **Errors, not surprises.** Library code returns a typed `Result` (`DomError`, `EngineError`); no `unwrap` / `expect` /
  `panic!` on a path a caller can reach. `expect` is allowed only for a genuinely impossible state, with a message
  saying why it can't happen. Trapped script panics are the one deliberate exception (`catch_unwind`, C-09).
- **Comments explain _why_, not _what_.** Cite the ADR/PRD/criterion a decision serves. Delete commented-out code.
- **DRY, and the Boy Scout Rule.** No copy-paste logic; leave every file you touch a little cleaner than you found it.

**Object Calisthenics (mechanically enforced):**

- No naked primitives in domain models — newtypes (`NodeId(u32)`, `TagName(String)`, `Px(f32)`, `Color(u32)`).
- First-class collections (`Children`, `AttributeMap`, `RuleSet`, `HeaderMap`) — no public `Vec` / `HashMap`.
- No `else` (early return / `match` / `if let`; `let … else` also counts).
- One level of indentation per function.
- One dot per line (Law of Demeter; builder chains are fine).
- No abbreviated names.
- Keep entities small (< ~100 lines, single responsibility).
- No public mutable fields — mutate through invariant-validating methods.

Hand-written `Display` + `std::error::Error` on domain errors keeps `domain/` free of a derive-macro dependency.

## SPDD Workflow

Feature work follows Structured Prompt-Driven Development (ADR-0007): `/spdd-analysis` (→ `spdd/analysis/`) →
`/spdd-reasons-canvas` (→ `spdd/prompt/`, the 7-stage REASONS canvas) → `/spdd-generate` → `/spdd-sync`. The skill
definitions live in `.agents/skills/` (`.cursor` is a symlink to `.agents`); Claude Code does not auto-discover that
directory, so read the relevant `SKILL.md` before following the flow. `docs/requirements/PRD-*.md` are the authoritative
inputs, `docs/adr/*.md` the constraints.

New architectural decisions get an ADR in `docs/adr/` (MADR format) plus a row in `docs/adr/README.md`.
