# ADR-0028: A Replaced Element's Natural Size Reaches Layout Through a `core/css` Port

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-11-14

---

## Context and Problem Statement

An `<img>` got its size from CSS alone (issue #83). The decoded image landed in `browser::ImageStore`, which only the
painter read, so layout never learned the image's natural size: the box kept whatever `core/css` guessed for a replaced
element, and an image the page did not size was stretched to that guess or never sized at all. Layout also had no way to
tell an image that is still loading from one that failed, which is what decides whether an `<img>` shows its `alt` text
(WHATWG HTML §15.4.2).

`core/css` must not depend on `core/browser` (the dependency already runs `browser → css`), and `LayoutEngine::layout`
is handed only a `StyledTree` and the viewport (`PRD-007:56-60`). The question: how does a resource's state reach the
layout engine?

---

## Decision Drivers

- `css` never sees where decoded images are kept; `browser` never sees layout (ADR-0026 keeps the siblings apart).
- `LayoutEngine` and `CascadeResolver` keep their signatures: the cascade stays a function of the DOM and the sheets, so
  an image arriving does not need a new cascade.
- The engine must distinguish "loaded, at this size", "still loading" and "will never load".
- Determinism (`ADR-0016`): the same inputs give the same `LayoutBoxTree`.

---

## Considered Options

- **Option 1**: A `NaturalSizes` port owned by `core/css`, injected into the layout engine when it is built (like
  `TextMeasurer`), answered by an adapter in `alloy` over `ImageStore`.
- **Option 2**: The cascade side — the source injected into `UaCascade`, and `StyledNode` carrying the natural size.
- **Option 3**: `browser::StoreSnapshot` implements the port directly.

---

## Decision Outcome

Chosen option: **Option 1**. Option 2 would make an arriving image re-run the cascade, which must stay a function of the
DOM and the sheets, and would change the closure signature every resolver passes. Option 3 would make `browser` name a
`css` layout port, coupling the two siblings ADR-0026 keeps apart, and would still need `alloy` to know the
node-to-image key.

- **The port.** `css::NaturalSizes::resource_of(&self, node: SnapshotId) -> ReplacedResource`, where `ReplacedResource`
  is `Loaded(NaturalSize)`, `Loading` or `Unavailable`. It is infallible: a node the source knows nothing about is
  `Unavailable`. `BlockLayout::new` uses `NoResources` (all `Unavailable`); `BlockLayout::with_natural_sizes` swaps a
  source in.
- **Not a whole-tree seam.** `PRD-007`'s whole-tree granularity rule is about the script-replaceable cascade and layout
  ports. `NaturalSizes`, like `TextMeasurer`, is a Rust-to-Rust lookup the engine makes while it lays out, never across
  a script boundary.
- **The adapter is a mapping.** `alloy/src/application/image_sizes.rs` maps `browser::ImageState` (`Loading`,
  `Decoded(size)`, `Failed`, or no entry) to `ReplacedResource`, keyed by the node index both sides already share. It
  does no I/O and is built per render from a borrowed store, so it lives in `alloy`'s application layer beside the
  pipeline and the painter — where `css` and `browser` already meet — not in `infrastructure/`, which `application` may
  not name (`arch-lint`).
- **`ImageStore` is tri-state.** A discovered image is `Loading`; a decoded one carries its pixels; a failed fetch or
  decode is recorded as `Failed` and damages the content, so the page is laid out again.
- **Layout owns the sizing rule** (CSS 2.1 §10.3.2, §10.6.2): the author's `width` / `height`, then the natural size,
  then — before the resource loads — the ratio of the element's own `width` / `height` attributes (WHATWG HTML §15.4.3),
  then the default: 0×0 for an `<img>`, 300×150 for every other replaced element. `LayoutBox` reports
  `IntrinsicSize::Natural` once loaded, `Pending` while loading and unpinned, `Resolved` otherwise.

### Consequences

- Good: an image the page does not size takes its natural size; one sized on one axis keeps its ratio;
  `<img width height>` reserves its box before it loads; the cascade is untouched when an image arrives.
- Good: the store's state also decides `alt` text, through the same port.
- Bad: `css::PORT_SCHEMA_VERSION` 14 → 15, not additive (`PRD-007` §6): unavailable replaced boxes report `Resolved`
  where they reported `Pending`.
- Bad: one virtual call per replaced box per layout — negligible next to the per-word `TextMeasurer` calls.

---

## Links

- Issue #83 and its decision comments; `docs/requirements/PRD-007-style-cascade-and-layout-engine-ports.md` §6
  (`14 → 15`); `docs/architecture/style-cascade-port-contract.md`
- `ADR-0010`, `ADR-0011`, `ADR-0016`, `ADR-0026`
