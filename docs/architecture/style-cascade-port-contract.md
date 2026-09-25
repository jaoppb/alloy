# `CascadeResolver` / `LayoutEngine` / `TextMeasurer` port — ADR-0011 contract record

The `CascadeResolver` / `LayoutEngine` / `TextMeasurer` seam in `core/css` is a **Replaceable Subsystem Port** under
`ADR-0011` (a "Mechanism + policy" port per the ADR's port table). This document is its contract record: the state of
all seven mandatory items at the `I3` freeze point (end of v0.5 B4).

| Item | Contract requirement                                                      | State                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ---- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1    | Seam PRD with variation + threat model                                    | ✅ `PRD-007` §2 (variation model: an author's style cascade and layout algorithm may be replaced without touching a consumer; §2 threat model: hostile author CSS, never a trusted-code boundary)                                                                                                                                                                                                                        |
| 2    | Port traits: assoc types only, no adapter types, object-safe or companion | ✅ All three traits (`CascadeResolver`, `LayoutEngine`, `TextMeasurer`) are already object-safe — no generic method, no associated type, every signature speaks only this crate's own types plus the shared `graphics` units. No companion needed. See §2 below                                                                                                                                                          |
| 3    | Boundary aggregates: domain-owned, `#[non_exhaustive]`, schema version    | ✅ `DomSnapshot`, `StyledTree`, `LayoutBoxTree`, `ComputedStyle`, `StyledNode`, `LayoutBox` all domain-owned in `core/css`, `#[non_exhaustive]`; `css::PORT_SCHEMA_VERSION` is the single version knob. **`= 3`** since B4 reshaped `ComputedStyle` / `StyledNode` / `LayoutBox` (additive fields; `PRD-007` migration note below)                                                                                       |
| 4    | Exactly one typed error, source location                                  | ✅ `CssError`, `#[non_exhaustive]`, one enum shared by all three traits; `CssStage` names which stage raised it, `SourceSpan` carries line/column where the parser produced one                                                                                                                                                                                                                                          |
| 5    | Written lifecycle & concurrency contract                                  | ✅ §5 below                                                                                                                                                                                                                                                                                                                                                                                                              |
| 6    | Conformance suite + reference adapter + `no-<adapter>`                    | ✅ `css::application::conformance::run_css_conformance`; `UaCascade` / `BlockLayout` / `FontBackedMeasurer` (real adapters) and `MockCascadeResolver` / `MockLayoutEngine` / `MockTextMeasurer` (reference mocks) both pass it (`tests/css_conformance.rs`). `--no-default-features` builds and tests the crate with no script-facing adapter linked (`core/css/Cargo.toml`'s `builtin-adapters` feature — see §6 below) |
| 7    | Frozen-API milestone                                                      | ✅ Frozen at `I3` (end of B4). `css::PORT_SCHEMA_VERSION = 3` is that surface; any future boundary change bumps it and adds a `PRD-007` migration note                                                                                                                                                                                                                                                                   |

---

## 2. Object-safety (item 2)

Unlike `RuntimeEngine` (`core/engine`), none of the three CSS traits needed a `dyn`-dispatch companion — they were
designed object-safe from `B0`:

- `CascadeResolver::resolve(&self, dom: &DomSnapshot, sheets: &StyleSheetSet) -> Result<StyledTree, CssError>`
- `LayoutEngine::layout(&self, styled: &StyledTree, constraints: &ViewportConstraints) -> Result<LayoutBoxTree, CssError>`
- `TextMeasurer::measure(&self, run: &TextRun, style: &ComputedText) -> Result<TextMetrics, CssError>`

Every parameter and return type is a concrete, `#[non_exhaustive]` boundary aggregate — no `impl Trait`, no generic
method, no associated type. `&dyn CascadeResolver`, `&dyn LayoutEngine` and `&dyn TextMeasurer` all compile and are
exactly what `core/css/src/infrastructure/layout/context.rs`'s `LayoutContext` holds
(`measurer: &'tree dyn TextMeasurer`) and what `application/conformance.rs` takes. `ADR-0011` item 2 is satisfied with
no companion trait, the same shape `graphics::RenderBackend` uses.

---

## 3. Boundary aggregates and the B4 schema bump

`css::PORT_SCHEMA_VERSION` moved `2 → 3` in v0.5 B4. Every change is **additive** — a new field or a new grouping, never
a removed or renarrowed one — so no existing `match` arm anywhere in the workspace needed to change:

| Aggregate       | What B4 added                                                                                                                                                                                    |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `ComputedStyle` | `border`, `width`, `height`, `box_sizing`, `text_align`, `white_space`, `flex` (grouped into `FlexStyle` per `ADR-0010` rule 7)                                                                  |
| `StyledNode`    | `text: Option<TextRun>`, `intrinsic_size: IntrinsicSize`                                                                                                                                         |
| `LayoutBox`     | Constructed from a `BoxEdges` (margin + border + padding grouped) instead of a bare margin, plus `intrinsic_size: IntrinsicSize`                                                                 |
| `TextMetrics`   | `baseline: Au` — the alignment line the inline formatting context's line boxes share; `TextMetrics::new` keeps the pre-B4 reading (baseline on the bottom edge) so a B3-era caller is unaffected |

`PRD-007` migration note: a consumer that pattern-matches `ComputedStyle`, `StyledNode` or `LayoutBox` by exhaustive
field list would have needed updating, but all three are `#[non_exhaustive]` (or accessed only through methods), so no
in-tree consumer needed a change. `core/css/tests/pipeline.rs`'s document-order assumption did change (a text node now
gets its own box — see `core/css/src/infrastructure/layout/block.rs`'s `push_own_text`), which is a **behavioural**
consequence of the new layout engine, not a boundary-aggregate break; the fix looks the box up by node
(`LayoutBoxTree::box_of`) rather than by a fixed index.

---

## 5. Lifecycle and concurrency contract (item 5)

### 5.1 Ownership of durable state

**The Skeleton (Rust) owns all durable state** (`ADR-0003`). None of the three adapters (`UaCascade`, `BlockLayout`,
`FontBackedMeasurer`) hold document state between calls — `UaCascade` caches only its own parsed, immutable UA
stylesheet (`assets/ua.css`, parsed once in `UaCascade::new`); `BlockLayout` and `FontBackedMeasurer` hold no state at
all beyond their measurer/font configuration. A `DomSnapshot`, `StyledTree` and `LayoutBoxTree` are plain immutable
values the caller owns; nothing survives inside an adapter between one `resolve`/`layout`/`measure` call and the next.

### 5.2 Threading model

- All three traits are `Send + Sync`. One adapter value may be shared across threads (`&dyn CascadeResolver`, etc.) with
  no interior mutability to race on.
- Every method takes `&self` — `resolve`, `layout` and `measure` may all be called concurrently from multiple threads on
  the same adapter instance, because each call is a pure function of its arguments (§5.3).
- There is no context object analogous to `ExecutionContext` on this port: a `CascadeResolver`/`LayoutEngine` call takes
  everything it needs as arguments and returns everything it produced as its `Result`. No handle to keep alive across
  calls, so no lifecycle beyond "construct the adapter once, call it as often as needed."

### 5.3 Purity and determinism

- `PRD-007:52` (cascade) and `:79-80` (layout): identical inputs — the same `DomSnapshot` + `StyleSheetSet`, or the same
  `StyledTree` + `ViewportConstraints` — MUST produce a field-for-field identical output. No adapter may read wall-clock
  time, randomness, or any ambient state.
- `core/css/src/application/conformance.rs` enforces this mechanically (`check_cascade_is_deterministic`,
  `check_layout_is_deterministic`, 100 repeated runs — `DETERMINISM_RUNS`). `core/css/tests/determinism.rs` runs the
  same 100-repetition proof against a document exercising all three B4 formatting contexts (margin collapse, inline
  text, Flexbox) together.
- Determinism is what makes re-entrancy moot: since no call reads or writes shared state, two calls on the same adapter
  from two threads never observe each other.

### 5.4 Re-entrancy and suspension

There is no suspend/resume point and no re-entrancy hazard: `resolve`/`layout`/`measure` are ordinary recursive
functions over an immutable input tree, never yielding control back to a caller-supplied callback mid-call. Unlike
`RuntimeEngine`'s native-binding re-entrancy concern, nothing on this port ever calls back into script or into another
port.

### 5.5 Cancellation

There is no cooperative cancellation. A call runs to completion or returns a typed `CssError` — see §5.6 for the one
category of built-in abort.

### 5.6 Resource ceilings and fault behaviour

- **Parse-time**: `infrastructure/parser/rules.rs`'s `MAX_NESTING_DEPTH` (32) refuses a stylesheet whose rule/`@media`
  nesting is hostile input rather than a real document, recovering with a `ParseNote` — the parser never panics on
  malformed or adversarial CSS text; it drops the offending declaration or rule and keeps going (`§2.8:350-354`,
  `core/css/tests/data/MANIFEST.md`).
- **Layout-time**: `infrastructure/layout/context.rs`'s `MAX_LAYOUT_DEPTH` (256) refuses a box tree nested past a depth
  no real page reaches, returning `CssError::Unsupported` instead of overflowing the native call stack.
- **Fault behaviour**: `CssError` is `#[non_exhaustive]` with `CssStage`-tagged variants (`UnknownNode`,
  `MissingComputedStyle`, `Unsupported`, …) and an optional `SourceSpan`. No adapter shipped in this crate calls
  `.unwrap()`/`.expect()`/`panic!` on a caller-reachable path (`clippy::unwrap_used` / `clippy::expect_used` /
  `clippy::panic` are workspace-denied outside test code and the two documented "genuinely impossible state" carve-outs
  — `ua_sheet.rs`'s embedded-asset parse and `conformance.rs`'s own assertions). A resolver/layout/measure failure is
  always a typed `Result::Err`, never a host-process abort — the same trapping discipline `PRD-003:62-70` establishes
  for the script engine, applied here to hostile author input instead of hostile script.
- There is no hot-reload story on this port yet (unlike `RuntimeEngine`'s §5.7): `UaCascade`/`BlockLayout` carry no
  swappable compiled artifact — a document is re-resolved and re-laid-out from scratch on every call, which is already
  the cheap path for a `DomSnapshot`-sized input.

### 5.7 Memory ceilings

Not enforced at this port: a pathological stylesheet or document can grow the `DomSnapshot`/`StyledTree`/`LayoutBoxTree`
arbitrarily. This mirrors `RuntimeEngine`'s v0.1 gap (`runtime-engine-port-contract.md` §5.5) and is deferred the same
way — the nesting-depth ceilings above already bound the pathological-recursion case, which is the sharper edge for a
tree-shaped input.

---

## 6. Conformance suite, reference adapters and `no-<adapter>` (item 6)

- `css::application::conformance::run_css_conformance(&dyn CascadeResolver, &dyn LayoutEngine)` — panics on the first
  violated rule, naming it. Checks: cascade determinism, layout determinism, whole-tree granularity (one call styles the
  whole snapshot, `PRD-007:78`), no foreign type escaping the `StyledTree`, and graceful handling of an empty document.
- `core/css/tests/css_conformance.rs` runs it against **both** pairs: the real adapters (`UaCascade` + `BlockLayout`)
  and the reference mocks (`MockCascadeResolver` + `MockLayoutEngine`, `infrastructure/mock.rs`) —
  `the_builtin_rust_adapters_pass_conformance` and `the_port_mocks_pass_conformance`. `core/css/tests/port_swap.rs`
  (`swapping_the_resolver_needs_no_change_to_the_other_aggregates`) additionally proves an adapter is swappable without
  touching the boundary aggregates.
- `core/css/Cargo.toml`'s `builtin-adapters` feature (default-on) gates nothing yet — every adapter this crate ships is
  Rust, and the scriptable `.rhai` cascade adapter of `PRD-007` §3.4 lives in `core/runtime/rhai-bindings`, never here.
  `cargo test -p css --no-default-features` therefore already builds and tests exactly the same code as the default
  feature set, which is the `no-<adapter>` proof this crate can offer today; the feature exists so a later phase that
  adds script-facing content has a switch to gate it behind.

---

## Audit

Re-run `cargo test -p css` (conformance is `tests/css_conformance.rs`; determinism is `tests/determinism.rs`),
`cargo test -p css --no-default-features` (item 6's `no-<adapter>` proof), and check `css::PORT_SCHEMA_VERSION` against
the last recorded value (`3`, as of this freeze) when reviewing any change to `ComputedStyle` / `StyledNode` /
`LayoutBox` / `CssError` / a trait signature (items 3/4/7). `just no-engine` additionally proves `core/css` links
neither `engine` nor `rhai`.
