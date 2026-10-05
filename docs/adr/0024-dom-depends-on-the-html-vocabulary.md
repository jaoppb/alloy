# ADR-0024: `dom` Depends on the `html` Vocabulary

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-10-05

---

## Context and Problem Statement

`TagName`, `AttributeName`, `AttributeValue` and `HtmlEntity` are HTML-standard vocabulary, yet they were defined in
`core/dom`, and `core/html` carried near-duplicates with diverging validation (`dom` rejected control characters,
whitespace and `" ' / = >` in attribute names; `html` only rejected empty names). `html` depended on `dom` (optional
`dom` feature) so it could ship `DomTreeSink` and `parse() -> DomTree`. v0.2 decision 2.1 made `dom` a "zero dependency"
crate. Issues #27, #28 and #29 ask for one definition, owned by `html`, and for consumers to import it from there.

---

## Decision Drivers

- One definition per concept; no silent divergence between two validators.
- `html` is the HTML standard's home; `dom` is node storage.
- No type leaks through re-exports (`ADR-0010`).

---

## Considered Options

- **Option 1**: Invert the dependency: `dom → html`. `html` is the leaf and owns the vocabulary, the tokenizer, the tree
  builder and the `TreeSink` port; `dom` owns `DomTreeSink` and `parse`.
- **Option 2**: Extract a third crate for the vocabulary — one more crate for four types.
- **Option 3**: Keep `html → dom` and move the vocabulary the other way — rejected, it is the problem being fixed.

---

## Decision Outcome

Chosen option: **Option 1**.

- `html` has no dependency on `dom` and no `dom` feature. Its only dependency is `thiserror`; adding another dependency
  to `html` flows into `dom` and every crate above it, and needs an ADR amendment.
- Constructors are location-free (`TagName::new(&str) -> Result<_, InvalidTagName>`); the tokenizer attaches the
  `SourceLocation`, `dom` converts the error into `DomError` through `From`.
- `AttributeName` has one, strict rule. An attribute whose name fails it is **dropped and reported** as
  `invalid-attribute-name` (`ADR-0023`), never silently and never fatally. This deviates from WHATWG, which keeps such
  attributes; the deviation is recorded in `PRD-008`.
- `dom` does not re-export the vocabulary; `css`, `rhai-bindings` and `alloy` depend on `html` directly.
- This supersedes v0.2 decision 2.1's "zero dependencies" for `dom`.

### Consequences

- Good: one definition of each type; the duplicated validation is gone.
- Good: `html` is a leaf, enforced by `arch-lint`.
- Bad: `dom` is no longer dependency-free; `html::PORT_SCHEMA_VERSION` 2 → 3.
- Bad: pages with attribute names containing `" ' / = >` lose those attributes (reported).

---

## Links

- `docs/architecture/html-tree-sink-port-contract.md`, `docs/requirements/PRD-008-html-tokenizer-and-tree-sink-ports.md`
  §6
- `ADR-0010`, `ADR-0011`, `ADR-0023`, issues #27, #28, #29
