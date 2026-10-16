# SPDD Analysis: Inline SVG and foreign content handling (issue #86)

## Original Business Requirement

[jaoppb/alloy#86](https://github.com/jaoppb/alloy/issues/86) — "html, css, alloy: inline svg and foreign content
handling". `<svg width="20" height="20"><text>hello</text></svg>` currently parses its children into ordinary HTML
namespace, causing `<text>` to be styled and laid out as an unknown element with text `"hello"` rendered. Furthermore,
`<svg><path/></svg><p>x</p>` fails to isolate the foreign content boundary, trapping subsequent elements. Acceptance
criteria:

1. `<svg width="20" height="20"><text>hello</text></svg>` does not render/paint text `hello`, and renders byte-identical
   to `<span style="display:inline-block;width:20px;height:20px"></span>`.
2. Test in `core/html` proves that `<svg><path/></svg><p>x</p>` results in `<path>` with foreign namespace and `<p>` in
   HTML as a sibling of `<svg>`, not as a child.
3. `cargo test -p html --test manifest_runner` passes with a foreign content line in `MANIFEST.md`.

## Current State (as found)

- `core/html`: `TreeSink::create_element` had signature `(tag: TagName, attributes: &AttributeList)`. Elements had no
  namespace tracking, defaulting everything to HTML.
- `core/dom`: `ElementData` and `DomTree` stored only `TagName` without a `Namespace` discriminator.
- `core/css`:
  - `DomSnapshot` and `StyledTree` lacked foreign element awareness.
  - Text inside `<text>` was collected into `TextRun` and rendered to ink.
  - `<svg>` in `ua.css` lacked default `display: inline-block`, defaulting to block with zero dimensions.
  - Presentational hints (`width`, `height`) on `<svg>` were not collected into `StyleSheetSet`.
  - Replaced elements with `IntrinsicSize::Pending` had no fallback 300×150 dimensions for `auto` sizing (CSS 2.2
    §10.3.2/§10.6.2).

## Decisions

1. **Namespace Domain Value Object**: Define `Namespace` enum in `core/html/src/domain/namespace.rs` with variants
   `Html`, `Svg`, `MathMl`. Methods: `is_foreign() -> bool`, `is_html() -> bool`, `as_str() -> &'static str`.
2. **Port Schema Version Bump**: Add `namespace: Namespace` parameter to `TreeSink::create_element`. Bump
   `html::PORT_SCHEMA_VERSION` 4 → 5. Update `PRD-008` migration table and `html-tree-sink-port-contract.md`.
3. **Foreign Content Stack & Breakout in TreeBuilder**: Track open elements with their namespace in `OpenElement`. When
   `<svg>` opens, push with `Namespace::Svg`. Children inside foreign elements inherit foreign namespace until closed or
   broken out by standard HTML tags.
4. **DOM Aggregate Updates**: Update `core/dom` `ElementData` and `DomTree` to store and expose `Namespace`, and
   implement `create_element_ns`.
5. **CSS Layout & Presentational Hints**:
   - `ua.css`: Add `svg { display: inline-block; }`.
   - `collect_sheets.rs`: Collect `width` and `height` attributes on `<svg>` into `presentational_hints` with
     `Origin::Author` and `Specificity::ZERO`.
   - `styled_tree.rs`: Nodes with `is_foreign_descendant() == true` do not generate text runs (`character_data_of`
     returns `None`).
   - `block.rs` / `inline.rs`: Foreign descendants do not generate layout boxes (`generates_box` returns `false`).
   - Replaced elements with `IntrinsicSize::Pending` do not layout inner flow children, and use fallback 300px width and
     150px height when dimensions are `auto`.
6. **Acceptance Verification**:
   - Manifest probe in `core/html/tests/manifest_runner.rs` verifying `<svg><path/></svg><p>x</p>`.
   - Golden render test in `alloy/tests/render_golden.rs` verifying byte-identical raster output between
     `<svg width="20" height="20"><text>hello</text></svg>` and
     `<span style="display:inline-block;width:20px;height:20px"></span>`.
