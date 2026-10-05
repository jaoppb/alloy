# SPDD Analysis: Move HTML vocabulary from `dom` into `html` (issue #28)

## Original Business Requirement

[jaoppb/alloy#28](https://github.com/jaoppb/alloy/issues/28) — "We implemented some things like `TagName`, `Attribute`
and so on that defines the core of the `html` things but at the `dom`. The goal of this issue is to map this technical
debt by moving the things that is from the HTML standard that is currently at the `dom` crate into the `html` crate and
also updating the usage from others crate to be directly pointing to the `html`."

Sub-issues: [#27](https://github.com/jaoppb/alloy/issues/27) (remove duplicated code between `html` and `dom`) and
[#29](https://github.com/jaoppb/alloy/issues/29) (update usages from `dom` to `html`).

## Current State (as found)

- `html → dom` through the default `dom` feature: `html::DomTreeSink` and `html::parse() -> dom::DomTree`. `dom` has no
  dependency besides `thiserror` (v0.2 report decision 2.1: "pure domain crate").
- `DomTree`'s own API names the HTML vocabulary: `create_element(TagName)`,
  `set_attribute(AttributeName, AttributeValue)`, `tag() -> &TagName`. `serialize_html` uses `HtmlEntity` and
  `TagName::is_void()`.
- Two near-identical `TagName` enums (`dom/src/domain/tag_name.rs`, `html/src/domain/tag.rs`) with diverged
  constructors, error types and helper methods.
- Two `AttributeName`s with **different validation**: `dom` rejects control/whitespace/`" ' / = >`; `html` only rejects
  empty. `DomTreeSink` silently drops (`let Ok(..) else { continue }`) any attribute `dom` rejects.
- `html::AttributeName::new_unchecked` (`token.rs`) bypasses validation entirely.

## Decisions

1. **Dependency inverts to `dom → html`.** `html` becomes the leaf: vocabulary, tokenizer, `TreeBuilder`, `TreeSink`
   port, no `dom` dependency, the `dom` cargo feature is removed (supersedes B5 requirement 5). `DomTreeSink` and
   `parse() -> DomTree` move into `dom` (e.g. `dom/src/infrastructure/html_sink.rs`). `serialize_html` stays in `dom`
   (it walks `DomTree`) and uses `html::HtmlEntity` / `html::TagName::is_void`.
2. **Location-free value-object constructors.** `html::TagName::new(raw) -> Result<TagName, InvalidTagName>` (small
   error type in `html`'s domain). The tokenizer wraps it into `HtmlError` with its `SourceLocation`; `dom` converts it
   via `From` into `DomError`. Same pattern for every moved value object.
3. **One strict `AttributeName`.** `dom`'s current rule (reject control, whitespace, `" ' / = >`; lowercase) is the only
   rule. `new_unchecked` is deleted. The parser drops a failing attribute **explicitly**, as a recoverable diagnostic —
   never silently, never fatally. Accepted, knowing WHATWG keeps such attributes.
4. **Scope = HTML vocabulary only.** Moves/dedupes: `TagName`, `AttributeName`, `AttributeValue`, `HtmlEntity`. Stays in
   `dom` as DOM-standard node storage: `AttributeMap`, `TextContent`, `CommentContent`, `ElementData`, `NodeKind`. Stays
   in `html` as token payloads: `AttributeList`, `Text`.
5. **`TagName` API pruned to parsing semantics.** Keep `as_str`, `is_void`, `is_rawtext`, `closes_paragraph`,
   `closes_list_item`, `is_heading`, `is_head_content`, `is_html`/`is_head`/`is_body`. Delete the `&str` free functions
   (`is_void_tag`, `is_rawtext_tag`, `is_block_tag`, `closes_paragraph`, `closes_list_item`, `is_heading_tag`) —
   `TreeBuilder` and the manifest runner call methods. Delete `is_replaced` (0 callers, rendering concern) and
   `is_block` (CSS UA-sheet concern). Convenience constructors = the union actually used by callers.
6. **No re-export.** `dom` does not re-export html types. `css`, `rhai-bindings` and `alloy` add an `html` dependency
   and import `html::TagName` etc. directly; `dom::TagName` stops existing.
7. **One PR closes #27, #29 and #28**, off `main`, every commit compiling:
    1. `docs(adr)`: ADR-0024 — `dom` depends on the `html` vocabulary (supersedes v0.2 decision 2.1's "zero
       dependencies").
    2. `refactor(html)`: merge `TagName`/`Attribute*`/`HtmlEntity`, prune API.
    3. `refactor(dom,html)`: invert the dependency, move `DomTreeSink` + `parse` into `dom`.
    4. `refactor(css,rhai-bindings,alloy)`: import vocabulary from `html`.
    5. `docs`: PRD-008 migration note, `html::PORT_SCHEMA_VERSION` 2→3, port contract record, `CLAUDE.md` crate map,
       `dom`/`html` `Cargo.toml` descriptions.
8. **Recoverable-diagnostics channel is a separate prerequisite issue
   ([#36](https://github.com/jaoppb/alloy/issues/36)).** `parse` returns the tree plus the recoverable parse errors
   (WHATWG-style); the `TokenSink`/`TreeSink` port contract is updated there. #28 is **blocked** until it merges; #28
   then only switches to the strict `AttributeName` and pushes rejections into that channel.

## Done (v1 of this change)

- `cargo tree -p html` shows no `dom`; `html` has no `dom` feature.
- No `dom::TagName`, `dom::AttributeName`, `dom::AttributeValue`, `dom::HtmlEntity` anywhere in the workspace; one
  definition of each, in `html`.
- No `&str` tag-predicate free functions and no `new_unchecked` in `html`.
- `alloy/tests/render_golden.rs` is byte-identical (rendered output today already drops the same attributes, so a pure
  move must not change pixels).
- `just gate` green; ADR-0024 + `docs/adr/README.md` row in the same PR.

## Non-goals

- Changing `AttributeMap`'s sorted (`BTreeMap`) serialization order to spec insertion order.
- Unifying `Text` with `TextContent`/`CommentContent`.
- Building the recoverable-diagnostics channel inside this PR (decision 8).
- Expanding the named-entity table (tracked in #31).
- Making `DomTree` generic over its element payload.

## Open questions

- None closed by refusal. Deferred to implementation: exact module path of the moved sink in `dom`
  (`infrastructure/html_sink.rs` proposed) and the name of the location-free error types (`InvalidTagName`,
  `InvalidAttributeName`).

## Risks

- **WHATWG deviation (decision 3).** Pages with attribute names containing `" ' / = >` lose those attributes. Accepted;
  must be stated in ADR-0024 and PRD-008, and surfaced through the diagnostics channel rather than hidden.
- **Blocked on a prerequisite (decision 8, [#36](https://github.com/jaoppb/alloy/issues/36)).** #28 cannot start its
  attribute work until the diagnostics issue exists and merges; the TagName/HtmlEntity move does not depend on it but
  ships in the same PR.
- **`dom` loses its "zero dependencies" property.** Any future `html` dependency (e.g. `tracing`) now flows transitively
  into `dom` and every crate above it; ADR-0024 must constrain `html`'s own dependency set.
- **Port contract churn.** `html::PORT_SCHEMA_VERSION` bumps (2→3, after #36) (public `DomTreeSink`, `dom` feature and
  free functions removed) while `docs/architecture/html-tree-sink-port-contract.md` is still unwritten — the bump has no
  record to land in unless that file is written in the same PR.
