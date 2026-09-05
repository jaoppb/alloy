# GEMINI.md

This file provides guidance to Google Gemini / Antigravity CLI when working with code in this repository.

## Current State

**v0.1–v0.2 delivered and merged to `main`** (`75dc34b`, `0a60036`; PRs #4–#6). v0.1 ("O engine vive", F0+F1+F2) built
the `RuntimeEngine`/`ExecutionContext` port (ADR-0011 Replaceable Port Contract path, not the PRD-002-verbatim path the
v0.1 report originally proposed — see `docs/reports/IMPLEMENTACAO-DETALHADA-V0-1.md`), the `RhaiEngine` backend, and
`alloy --script`. v0.2 ("DOM scriptável e contido", F3+I1+F6, closes C-03/C-06/C-07/C-08/C-09) added `core/dom` (pure
arena domain crate), the scriptable-DOM bridge (`NodeHandle` self-guarding bindings in `core/runtime/rhai`), the
object-safe `dyn` companion (ADR-0013), and `run_with_fallback` (primary script → embedded default script on a clean
tree → Rust `minimal_document()`). `engine::PORT_SCHEMA_VERSION` moved 1→2 here. Full detail:
`docs/architecture/runtime-engine-port-contract.md`, `docs/reports/IMPLEMENTACAO-DETALHADA-V0-2.md`.

**v0.3 ("primeiro pixel", F4a) delivered and merged to `main`.** `core/graphics`: `DisplayList` (PRD-005's six
commands), the `RenderBackend` port + conformance suite, `SoftwareCpuBackend` (integer-exact coverage/clipping, no GPU
dependency — Vulkan/OpenGL tiers are ADR-0009's roadmap, not yet implemented), a dependency-free PNG encoder and
golden-image gate, and real text rasterization (`FontProvider`, `DrawText`, `TextMeasurer`, `ttf-parser` for font
parsing). `graphics::PORT_SCHEMA_VERSION = 2`. `docs/reports/IMPLEMENTACAO-DETALHADA-V0-3.md`.

**v0.5 ("página real") is in progress on `feat/v0-5`, not yet merged to `main`.** Full phase-by-phase status, evidence
and DoD checklists: `docs/v0-5-handoff/README.md` (the phase package) and `docs/reports/V0-5-PROGRESSO-E-PENDENCIAS.md`.
Delivered and independently verified (`cargo test`/`clippy` per crate, this session):

- **B4 — `core/css`**: a full layout engine over the B0–B3 port skeleton — box model (`BoxEdges`, margin collapse,
  `box-sizing`), a complete inline formatting context (word segmentation, whitespace collapse, `text-align` incl.
  `justify`, baseline alignment), and Flexbox (CSS Flexbox L1 §9, single-line; multi-line `flex-wrap` documented as a
  cut in `core/css/tests/data/MANIFEST.md` for v0.7). `css::PORT_SCHEMA_VERSION = 3`. Frozen at integration point `I3`:
  `docs/architecture/style-cascade-port-contract.md`.
- **B5 — `core/html`**: HTML5 tokenizer + `TreeSink`/`TokenSink` ports (PRD-008) over `dom::DomTree` — WHATWG §13.2.5
  state machine for the states a real document uses, `<script>`/`<style>` raw-text mode, minimal tag-omission rules.
  `core/html/tests/data/MANIFEST.md` + `manifest_runner.rs` mirror B1's two-way consistency gate. **Gap found this
  session**: `HtmlError` carries no source location (line/column) — ADR-0011 item 4 is not fully met here yet.
- **X — `core/graphics`**: PNG decoder (`IHDR`/`IDAT`/`IEND`, RGB/RGBA 8-bit) over `network::inflate`, `DrawImage` on
  `SoftwareCpuBackend`, integer box-sample scaling (ADR-0016 — no floating-point in the geometry). Fuzz targets exist
  (`fuzz/fuzz_targets/{inflate,png_decode}.rs`) but were not run for 10 minutes/target in this session — CI's new `fuzz`
  job is the first place that will actually prove it.
- **I2 — `alloy`**: the full headless pipeline,
  `bytes → html::parse → css cascade/layout → paint → DisplayList → SoftwareCpuBackend → png::encode`, wired in
  `alloy/src/application/{pipeline,paint}.rs` behind `alloy render <file.html> -o <out.png>`.
  `alloy/tests/render_golden.rs` proves byte-identical determinism over 100 runs. **Gap found this session**: the
  phase's own checkpoint (`git push` + PR draft) never happened — `feat/v0-5` has no remote counterpart and no PR exists
  yet.
- **M — `core/runtime/rhai-bindings`**: `NETWORK_BINDINGS`/`WINDOW_BINDINGS` (same self-guarding-per-method pattern as
  `dom_bindings.rs`) plus a scriptable `.rhai` cascade adapter, all mapping errors through the generalized
  `EngineError::Subsystem` (Phase EE, `engine::PORT_SCHEMA_VERSION` 2→3, `Dom` kept `#[deprecated]` not removed —
  `PRD-002` §4.2/§4.5). `rhai-bindings/tests/fault_injection.rs` covers the panic matrix for all three binding tables.
  **Gap found this session, now fixed in Phase P**: the DoD's "committed benchmark baseline" for
  `core/runtime/rhai/benches/hook_overhead.rs` was never actually committed — a new baseline file and the blocking
  `hook-benchmark` CI job close that.
- **C0/C1 — `core/network`**: hand-written HTTP/1.1 client, `rustls`+`ring` TLS (the C0 spike found the pure-Rust
  RustCrypto alternative NO-GO — `docs/reports/SPIKE-C0-TLS-PROVIDER.md`), `HttpTransport`/`RequestPolicy` ports
  (`PRD-009`). `network::PORT_SCHEMA_VERSION = 1`, **not yet frozen** — freezes at `I4`.
- **C2 — `core/window`**: `winit`+`softbuffer` adapter, `WindowSystem`/`Presenter` ports (`PRD-010`), headless reference
  (`HeadlessWindowSystem`/`RecordingPresenter`). `window::PORT_SCHEMA_VERSION = 1`, **not yet frozen**.
- **EE — `core/engine`**: `EngineError::Subsystem { subsystem: SubsystemName, .. }` generalizes the v0.2 `Dom` variant
  to cover Css/Graphics/Network/Window uniformly (`PRD-002` §4.5).
- **P (this phase, docs/CI only)**: ADR-0018 (`unsafe` by threat surface) and ADR-0019 (single event loop) → `Accepted`;
  N-02 rewritten in `PRD-001`; `PRD-009`/`PRD-010` written; `http-transport-port-contract.md` and
  `window-system-port-contract.md` written (both honestly marked "not yet frozen" — `I4` hasn't happened).
  `html-tree-sink-port-contract.md` (the fourth item-4 contract record) is **still outstanding** —
  `core/html/src/lib.rs` already references it and carries `PORT_SCHEMA_VERSION`, but the record itself was not written
  in this session; it should also document the `HtmlError` source-location gap above. New blocking CI gates:
  `hook-benchmark`, `unsafe-audit` (flipped from advisory — `ci/unsafe_audit.sh` scans direct dependencies via
  `cargo-geiger` JSON output against `unsafe-allowlist.toml`; found and allowlisted a real gap,
  `tracing`/`tracing-subscriber`, which don't cleanly fit any of ADR-0018's three nominal rows — flagged there for a
  future ADR revision), `css-conformance` (extended to `-p html`), `layering` (renamed from `no-engine`, `no-engine`
  kept as a `just` alias, extended to `core/html`), `fuzz` (`{inflate, png_decode, css_parse}`, wiring only — not run to
  completion in this sandbox: no `cargo-fuzz`/nightly toolchain available here), `coverage` (extended to
  `css`/`network`/`window`/`html` `domain/` only, via `--ignore-filename-regex`). **Known gap this session did not
  close**: that coverage extension currently measures **~66% lines**, not the 85% the gate requires — it is wired as
  specified and will fail until more domain-level tests are written for `network` (`Url`, `HeaderMap`, error variants)
  and `window` (`error`, `attributes`) especially.
- **I4 — `alloy <url>`, native window rendering**: **not part of this delivered set** — in progress separately.

Still **stubs** (a doc comment and `#![forbid(unsafe_code)]`, no functions): `core/js`, `devtools`, `extension`. Open
criteria: C-10 … C-18, F10/F11 (v0.7+). Follow the `domain/` / `application/` / `infrastructure/` layering every
delivered crate now demonstrates; `docs/adr/` + `docs/requirements/` remain the authoritative contract.

## Commands

Tooling is split: Cargo for Rust, pnpm for Markdown quality gates. A root `justfile` wraps both — `just` lists every
recipe.

```bash
just gate                                    # full local gate (fmt-check + clippy + check + test + deny + coverage + arch + layering + css-conformance + unsafe-audit) — mirrors CI minus fuzz/hook-benchmark/fault-injection
just setup                                   # one-time: pnpm deps, rust components, cargo-deny, cargo-llvm-cov, arch-lint, git hooks
just test engine "-- name"                   # scoped test run
just run --script scripts/hello.rhai         # run the alloy binary
just deny | just coverage | just layering    # individual CI gates (`no-engine` still works as an alias for `layering`)
just hook-benchmark | just fuzz [target]     # not in `just gate` (slow/hardware-noisy) — CI-only otherwise, `fuzz` needs nightly + cargo-fuzz

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

| Path                         | Package         | Responsibility                                                                                                            |
| ---------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `core/engine`                | `engine`        | `RuntimeEngine` / `ExecutionContext` traits, `EngineValue`, capability bitflags. **Zero dependencies — pure abstraction** |
| `core/runtime/rhai`          | `rhai-runtime`  | Concrete Rhai backend implementing `engine`'s traits — names no domain crate                                              |
| `core/runtime/rhai-bindings` | `rhai-bindings` | Domain-coupled script bridges (DOM/CSS/network/window bindings) split out of `rhai-runtime` (v0.5 "R")                    |
| `core/js`                    | `js`            | Web-content ECMAScript runtime (untrusted page `<script>`) — distinct from the Rhai muscle engine. **Stub (v0.7+)**       |
| `core/dom`                   | `dom`           | Node hierarchy, elements, mutations                                                                                       |
| `core/html`                  | `html`          | HTML5 tokenizer & `TreeSink`/`TokenSink` ports over `core/dom` (v0.5 B5)                                                  |
| `core/css`                   | `css`           | Parser, selectors, cascade, box model / inline formatting / Flexbox layout (v0.5 B0–B4)                                   |
| `core/graphics`              | `graphics`      | `DisplayList`, `RenderBackend` port, `SoftwareCpuBackend`, PNG codec, text rasterization (v0.3–v0.5)                      |
| `core/window`                | `window`        | `WindowSystem` / `Presenter` ports, `winit`+`softbuffer` adapter, headless reference (v0.5 C2)                            |
| `core/network`               | `network`       | `HttpTransport` / `RequestPolicy` ports, hand-written HTTP/1.1 + `rustls`/`ring` TLS (v0.5 C0/C1)                         |
| `devtools`                   | `devtools`      | Debug protocol, inspector, hot-reload orchestration. **Stub (v0.9+)**                                                     |
| `extension`                  | `extension`     | WebExtensions bridge. **Stub**                                                                                            |

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

Feature work follows Structured Prompt-Driven Development (ADR-0007). The skill definitions live in `.agents/skills/`
and are auto-discovered by Gemini. The 4-phase pipeline:

1. `spdd-analysis` skill (→ `spdd/analysis/`) — strategic context enrichment from PRDs and codebase
2. `spdd-reasons-canvas` skill (→ `spdd/prompt/`) — 7-stage REASONS canvas (Requirements, Entities, Approach, Structure,
   Operations, Norms, Safeguards)
3. `spdd-generate` skill — code generation strictly following the canvas Operations sequence
4. `spdd-sync` skill — bidirectional sync between code changes and the prompt file

Read the relevant `SKILL.md` in `.agents/skills/spdd-*/` before following the flow. `docs/requirements/PRD-*.md` are the
authoritative inputs, `docs/adr/*.md` the constraints.

New architectural decisions get an ADR in `docs/adr/` (MADR format) plus a row in `docs/adr/README.md`.

## Key References

| Category               | Path                                         |
| ---------------------- | -------------------------------------------- |
| ADRs (decisions)       | `docs/adr/*.md`                              |
| PRDs (requirements)    | `docs/requirements/PRD-*.md`                 |
| Port contracts         | `docs/architecture/*-port-contract.md`       |
| Implementation reports | `docs/reports/IMPLEMENTACAO-DETALHADA-*.md`  |
| v0.5 handoff package   | `docs/v0-5-handoff/`                         |
| SPDD analysis          | `spdd/analysis/`                             |
| SPDD prompts           | `spdd/prompt/`                               |
| CI pipeline            | `.github/workflows/ci.yml`                   |
| CI scripts             | `ci/hook_benchmark.sh`, `ci/unsafe_audit.sh` |
| Arch lint rules        | `arch-lint.toml`                             |
| Unsafe allowlist       | `unsafe-allowlist.toml`                      |
