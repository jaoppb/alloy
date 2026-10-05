# Move the HTML vocabulary from `dom` into `html` (issues #27, #28, #29)

## Requirements

`TagName`, `AttributeName`, `AttributeValue` and `HtmlEntity` have one definition, in `html`. The dependency is
`dom → html`; `DomTreeSink` and `parse` live in `dom`. Consumers import the vocabulary from `html`. Done:
`cargo tree -p html` shows no `dom`, no `dom::TagName`-style path remains, `render_golden.rs` byte-identical, gates
green.

## Entities

`html::{TagName, AttributeName, AttributeValue, HtmlEntity}` (value objects, location-free constructors with
`InvalidTagName` / `InvalidAttributeName`); `dom::{DomTree, AttributeMap, ElementData, DomTreeSink, ParseOutcome}`.

## Approach

Analysis: `spdd/analysis/202610050000-[Analysis]-move-html-vocabulary-from-dom-to-html-issue-28.md`; ADR-0024. The
tokenizer resolves named references through `HtmlEntity`; invalid attribute names are dropped with
`invalid-attribute-name` (replaces `unsupported-attribute-name`).

## Structure

`core/html` (leaf) ← `core/dom` ← `core/css`, `rhai-bindings`, `alloy`; `css`/`rhai-bindings`/`alloy` also depend on
`html`.

## Operations

1. Merge vocabulary into `html`, strict `AttributeName`, drop `&str` predicates and `new_unchecked`.
2. Invert the dependency, move the sink and `parse` into `dom`, migrate consumers.
3. Docs: PRD-008 v3 row, port contract, `CLAUDE.md`.

## Norms

Object Calisthenics and Clean Code per `CLAUDE.md`; `thiserror` errors; no `println!`.

## Safeguards

`arch-lint`: `html` must not depend on or import `dom`. `html::PORT_SCHEMA_VERSION` 2 → 3. `html` dependencies stay
`thiserror` only.
