# ADR-0023: Recoverable Diagnostics Are Port Data, Not Errors

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-10-05

---

## Context and Problem Statement

`ADR-0011` item 4 requires "exactly one `enum <Port>Error` per port". For `core/html` that error, `HtmlError`, aborted
the whole parse, so a single malformed construct failed the document or — where a sink worked around it — was dropped
silently (`DomTreeSink` skipped attributes `dom` rejected). WHATWG HTML treats almost all malformations as _parse
errors_ that are reported while parsing continues. Issue #36 needs a way to report them without aborting, and #28 (a
strict `AttributeName`) would otherwise turn one bad attribute into a failed page.

The question: where does a recoverable, located error live, given that item 4 allows only one error enum?

---

## Decision Drivers

- A recoverable condition must never abort a parse, and must never be dropped without a trace.
- The port keeps exactly one fatal error type (`ADR-0011` item 4, `ADR-0015`).
- Diagnostics must stay in order with the tree operations they relate to, including across suspend/resume (`PRD-008`
  §3.4).
- Replaceable sinks must be able to collect or ignore diagnostics without being able to re-introduce an abort.

---

## Considered Options

- **Option 1**: Recoverable diagnostics are **port data**: a located, coded value object delivered through the port
  (`Token::ParseError`, `TreeSink::parse_error`), distinct from the fatal error, which still travels as `Err`.
- **Option 2**: A collector owned by the parser and returned beside the result — needs a second channel for sink-side
  conditions and hands back partial state on suspension.
- **Option 3**: Reuse the error enum — nothing in the type says whether a variant aborts or continues.

---

## Decision Outcome

Chosen option: **Option 1**.

- **Fatal vs. recoverable.** `<Port>Error` (here `HtmlError`) is reserved for failures a caller cannot recover from:
  adapter/sink failure and value-object constructor validation. A recoverable condition is a `ParseDiagnostic`
  (`code: ParseErrorCode`, `location: SourceLocation`), `thiserror`-derived, with a closed `#[non_exhaustive]` code
  vocabulary using WHATWG names where one exists.
- **In-band delivery.** The tokenizer emits `Token::ParseError` _before_ the token that triggered it; the tree builder
  forwards it to a required `TreeSink::parse_error`. Ordering and suspend/resume need no extra plumbing.
- **Infallible.** `parse_error` returns `()`: a sink may record or ignore a diagnostic but can never turn it into an
  abort.
- **Sinks own collection.** The default adapter returns them as `ParseOutcome { tree, diagnostics }`; the mock records
  them as events. The list is unbounded (memory linear in input).
- **Gate.** Every code is listed in the crate's manifest and has an exact-location probe.

Item 4 is therefore read as: one _fatal_ error enum per port; recoverable diagnostics are boundary data under item 3
(owned by the domain crate, `#[non_exhaustive]`, covered by the schema version).

### Consequences

- Good: a malformed page can no longer fail whole; nothing is silently dropped; diagnostics carry exact line/column.
- Good: the pattern is reusable by other ports with lenient input (e.g. the CSS parser's recovery notes).
- Bad: `TreeSink` gained a required method and `Token` a variant — `html::PORT_SCHEMA_VERSION` 1 → 2.
- Bad: tokens and attribute entries now carry a location (ignored by equality), a larger aggregate.

---

## Links

- `docs/architecture/html-tree-sink-port-contract.md` (§3–§4),
  `docs/requirements/PRD-008-html-tokenizer-and-tree-sink-ports.md` §6
- `ADR-0011`, `ADR-0015`, issue #36 (and #28, which this unblocks)
