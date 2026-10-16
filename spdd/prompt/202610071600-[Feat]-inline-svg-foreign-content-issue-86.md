# Inline SVG and foreign content handling for `html`, `dom`, `css` and `alloy` (issue #86)

## Requirements

1. `<svg width="20" height="20"><text>hello</text></svg>` does not render/paint text `hello`, and renders byte-identical
   to `<span style="display:inline-block;width:20px;height:20px"></span>`.
2. Test in `core/html` proves that `<svg><path/></svg><p>x</p>` results in `<path>` with foreign namespace and `<p>` in
   HTML as a sibling of `<svg>`, not as a child.
3. `cargo test -p html --test manifest_runner` passes with a foreign content line in `MANIFEST.md`.

## Entities

- `Namespace` (`Html`, `Svg`, `MathMl`) in `core/html/src/domain/namespace.rs`.
- `TreeSink::create_element` updated with `namespace: Namespace`.
- `PORT_SCHEMA_VERSION = 5` in `core/html/src/lib.rs`.
- `ElementData` and `DomTree` in `core/dom` updated with `namespace: Namespace` and `create_element_ns`.
- `DomSnapshot` / `NodeRef` in `core/css` exposing `namespace()`, `is_foreign()`, and `is_foreign_descendant()`.
- `StyleSheetSet` exposing `push_presentational_hint` and `presentational_of`.

## Approach

- Parse foreign content into appropriate namespaces in `core/html` tree builder, handling breakout tags back to HTML.
- Propagate namespace to `core/dom` and `core/css` snapshot.
- Collect presentational hints (`width`, `height`) from `<svg>` into author declarations with `Specificity::ZERO`.
- Set `svg { display: inline-block; }` in `ua.css`.
- Replaced elements with pending intrinsic size suppress child flow formatting and adopt CSS 2.2 §10.3.2/§10.6.2
  fallback dimensions (300×150 Au) when sized `auto`.
- Foreign descendants do not generate layout boxes or text runs.

## Operations

1. Implement `Namespace` domain enum and update `TreeSink` signature.
2. Update tree builder to track foreign namespaces and HTML breakout tags.
3. Register `"foreign content"` in `MANIFEST.md` and `SUPPORTED_SYNTAX`.
4. Update `core/dom` DOM tree aggregate with `namespace` tracking.
5. Update `core/css`: `ua.css`, `collect_sheets.rs`, `author_rules.rs`, `styled_tree.rs`, `block.rs`, and `inline.rs`.
6. Add acceptance tests in `manifest_runner.rs` and `render_golden.rs`.
7. Update `PRD-008` migration table and contract record.

## Norms

- Object Calisthenics: no `else`, first-class collections, 1 indentation level, no naked primitives.
- `thiserror` for error handling, `tracing` for logging.
- `just gate` passes without warnings.
