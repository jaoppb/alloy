# PRD-007: Style Cascade and Layout Engine Ports

- **Status**: Proposed
- **Author**: Core Architecture Team
- **Date**: 2026-08-28
- **Target Release**: v0.5

---

## 1. Executive Summary

CSS **parsing** (tokenizer, selector syntax, rule sets) stays native Rust in `core/css`. The **cascade resolution** and
**layout** stages of the pipeline are exposed as replaceable ports, so an engine developer can substitute a custom
specificity/inheritance resolver or a custom layout algorithm — in Rust, or as a `.rhai` / Wasm adapter driven through
`RuntimeEngine` — without modifying `core/dom`, `core/graphics`, or any consumer. This PRD conforms to the Replaceable
Port Contract of `ADR-0011` and realises the goals stated in `PRD-001:27` and `PRD-001:62`.

---

## 2. Problem Statement

1. Cascade and layout are the policy-heavy stages of `HtmlStream → DomTree → StyledTree → LayoutBoxTree → DisplayList`
   (`ADR-0010:114-117`), and are the explicit target of the "swap the algorithm" goals — yet the pipeline exposes no
   seam there today.
2. A naive seam built from per-node FFI callbacks would violate the `<10μs` per-hook budget (`PRD-001:96`) and Object
   Calisthenics rule 3 (`ADR-0010:131`) in the hot path.
3. Without a frozen boundary aggregate, any port trait written now is rewritten when `StyledTree` changes.

---

## 3. Architecture & Port Specifications

### 3.1 Boundary aggregates (owned by `core/css`, `#[non_exhaustive]`, versioned)

- `DomSnapshot` — an immutable, read-only projection of `DomTree` (elements, attributes, tree shape). No `core/dom`
  internal type leaks; it is produced by an explicit mapping function.
- `StyleSheetSet` — parsed, ordered rules with origin (`UserAgent`, `User`, `Author`). Produced by the native Rust
  parser; not replaceable in this PRD.
- `StyledTree` — computed value per node after the cascade.
- `LayoutBoxTree` — boxes with resolved geometry, ready for `DisplayList` generation.

### 3.2 `CascadeResolver` trait (`css/application/ports.rs`)

```rust
pub trait CascadeResolver: Send + Sync {
    fn resolve(&self, dom: &DomSnapshot, sheets: &StyleSheetSet)
        -> Result<StyledTree, CssError>;
}
```

Whole-tree in, whole-tree out — coarse granularity is **mandated**, not optional. The resolver is pure and
deterministic: identical inputs produce an identical `StyledTree`.

### 3.3 `LayoutEngine` trait (`css/application/ports.rs`)

```rust
pub trait LayoutEngine: Send + Sync {
    fn layout(&self, styled: &StyledTree, constraints: &ViewportConstraints)
        -> Result<LayoutBoxTree, CssError>;
}
```

### 3.4 Script-driven adapters

When an adapter is a `.rhai` or Wasm script it runs through `RuntimeEngine` (`PRD-002`). This requires `DomSnapshot` and
`StyledTree` to be registered as engine types (`CustomType`), which depends on `C-03`. A script adapter is granted
`DOM_READ | GRAPHICS_DRAW` and **never** `DOM_MUTATE`, matching the CSS/Style profile of `PRD-003:56`.

### 3.5 Reference implementations

The built-in Rust cascade resolver and the built-in flow-plus-Flexbox layout engine are themselves adapters behind these
ports — the contract is dogfooded, not bypassed for the default path.

---

## 4. Requirements & Invariants

1. **No per-node callbacks** cross the seam; the unit of exchange is the whole tree.
2. **Determinism**: the same `DomSnapshot` + `StyleSheetSet` yields a byte-identical `LayoutBoxTree`, verified by golden
   images on `SoftwareCpuBackend` and by rectangle-assertion tests (`roadmap §5`).
3. **Fallback**: a script adapter that errors, panics, or exceeds its instruction budget falls back to the built-in Rust
   adapter, and the page still renders (`PRD-003:66-69`).
4. **No foreign types**: no `core/dom` or `core/graphics` internal type appears in a port signature or a boundary
   aggregate.
5. **Contract compliance**: this port satisfies all seven items of `ADR-0011`, including the `no-script` feature (Rust
   adapters only) and the `css-conformance` target.

---

## 5. Acceptance Criteria

- [ ] `CascadeResolver`, `LayoutEngine`, and the four boundary aggregates defined in `core/css`, frozen at integration
      point `I3`.
- [ ] Built-in Rust cascade and layout adapters pass the `css-conformance` suite.
- [ ] A mock `CascadeResolver` swaps in and changes computed styles **without changing** `core/dom` or `core/graphics`.
- [ ] A `.rhai` cascade adapter alters a computed property and the screen repaints, with capability limited to
      `DOM_READ | GRAPHICS_DRAW`.
- [ ] A script adapter that panics falls back to the built-in resolver and the page still renders.
- [ ] `core/css` builds and tests with `--no-default-features` (feature `no-script`), using only Rust adapters.
- [ ] Determinism test: 100 repeated runs of the same input produce the identical `LayoutBoxTree`.

---

## 6. Post-freeze migration notes

The boundary aggregates and `css::PORT_SCHEMA_VERSION` froze at `I3` (end of B4, version `3`). Every later change is
recorded here. Up to version `7` each was **additive** — a new `#[non_exhaustive]` field or grouping, never a removed or
renarrowed one, so no in-tree consumer's `match` needed updating; `7 → 8` is the first that is not, and its note says
what a consumer has to change.

### `3 → 4` — fonts increment: `ComputedStyle::font_family`

`ComputedStyle` gains `font_family: FontFamilyList` (`core/css/src/domain/computed/font.rs`), an inherited property (CSS
Fonts L4 §5.1) added alongside `color` / `font_size` in `ComputedStyle::inheriting_from`. It is a fixed-capacity `Copy`
list rather than a `Vec` because `ComputedStyle` was `Copy` (until `8`) and copied per node during layout; the two size
cuts (`FontFamilyList::CAPACITY` families, `FamilyName::CAPACITY` bytes per name) are declared in
`core/css/tests/data/MANIFEST.md` beside the Flexbox cuts. Consumers read it through `ComputedStyle::font_family()`; a
consumer that does not care about fonts is unaffected.

### `4 → 5` — "unstyled real sites" follow-up: the `background` / `border` shorthands

`SUPPORTED_PROPERTIES` grows from 34 to 36: the CSS Backgrounds & Borders L3 `background` and `border` shorthands are
now accepted by the parser and resolved by the cascade, each **narrowed to the single component this cut already has a
computed value for** — `background` → its colour (folded into `ComputedStyle::background_color`), `border` → its width
(folded into the `border` edges, like `border-width`). `url()` layers, gradients, position/repeat/size keywords, and a
border's `<line-style>` and colour are scanned past; `none` / `0` clear. The narrowings are declared in
`core/css/tests/data/MANIFEST.md` beside the Flexbox and font cuts.

**No boundary-aggregate field changed** — this is a wider set of _inputs_ for fields that already exist, so a consumer
that pattern-matches `ComputedStyle` is unaffected; a producer feeding real-world stylesheets simply gets
`background_color` / `border` populated from declarations it previously dropped. Brought forward from the planned v0.7
CSS widening because the blank-window fix (`docs/reports/DIAGNOSTICO-JANELA-BRANCA-WAYLAND.md`) left real pages visibly
unstyled and the page background was the highest-leverage single gap. `margin: auto` centring and `background-image`
fetch/paint are the next items and are **not** in this bump.

### `7 → 8` — PR #19 review round: `ComputedStyle` loses `Copy`, schema-7 boundary types settled

The review of the schema-7 widening changed boundary types a consumer can observe. Two of the changes are **not
additive**:

- **`ComputedStyle` is `Clone`, no longer `Copy`.** Its CSS Grid group (`GridStyle`, ~2.9 KB of fixed-capacity track
  lists, area names and line names) was inlined in schema 7 and made every style ~3.7 KB, copied through each cascade
  step and stored per `StyledNode`, for data no layout reads yet (`display: grid` is rejected). It now sits behind a
  shared `std::sync::Arc` that stays `None` while every grid property is `initial`, so a grid-less node allocates
  nothing: `ComputedStyle` measures 856 bytes (was 3756) and `StyledNode` 920 (was 3824) on a 64-bit target, a budget
  `core/css/tests/computed_style_footprint.rs` pins. `ComputedStyle::grid()` lends `&GridStyle` instead of returning it
  by value; the `with_*` builders and `inheriting_from` are no longer `const fn` (`initial()` still is). The
  `pass-by-value-size-limit = 4096` override that hid the cost from clippy is removed from `clippy.toml`. **Migration:**
  replace `*node.style()` with `node.style().clone()` (or keep the borrow), and copy the grid group explicitly
  (`*style.grid()`) where a by-value `GridStyle` is needed.
- **`TrackList::from_tracks` answers `Option<TrackList>`**, refusing more than `TrackList::CAPACITY` (16) tracks instead
  of truncating them. **Migration:** handle the `None`.

The rest is additive or confined to the cascade helpers:

- `Display` gains `InlineBlock` and `ListItem` (it is `#[non_exhaustive]`, so an outside `match` already has a wildcard
  arm); the closed `InputType` vocabulary is new.
- `VariableError` gains `ExpansionLimit`. `VariableError` is **not** `#[non_exhaustive]`, so an exhaustive `match` on it
  outside `core/css` needs the new arm.
- The public cascade helpers changed shape: `text_values::apply` takes the parent's `font-weight` (what `bolder` /
  `lighter` are relative to), and `grid_values::{apply, reset, inherit}` and
  `logical_values::{apply, apply_with_context}` borrow the style they read instead of taking it by value.
- Custom properties and `var()` are cascaded, in a per-node side table outside `ComputedStyle`; their semantics (and
  those of logical properties and relative font weights) are recorded in
  `docs/architecture/style-cascade-port-contract.md`, "Cascade semantics of schema 7".
