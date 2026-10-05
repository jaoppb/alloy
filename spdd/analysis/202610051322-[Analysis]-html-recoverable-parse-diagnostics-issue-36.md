# SPDD Analysis: `html` recoverable parse-diagnostics channel (issue #36)

## Original Business Requirement

Source: <https://github.com/jaoppb/alloy/issues/36> (body after two grill rounds, verbatim).

### Context

Prerequisite for jaoppb/alloy#28. **Blocks #28.**

Today `html` has only one error path: `HtmlError` returned through `Result`, which aborts the parse. Two problems follow
from that:

- `core/html/src/infrastructure/tokenizer/attribute_state.rs:23` does
  `AttributeName::new(name_str, cursor.location())?`. Once #28 makes `AttributeName` strict (it rejects control
  characters, whitespace and `" ' / = >`), one malformed attribute such as `<div a"b=1>` would fail the **whole
  document**.
- `DomTreeSink` currently hides that mismatch by **silently** dropping attributes `dom` rejects
  (`let Ok(..) = dom::AttributeName::new(..) else { continue };`).

WHATWG treats these as _parse errors_: they are reported, and parsing continues.

### Goal

A non-fatal diagnostics channel. `parse` returns the built tree **plus** the recoverable parse errors it found, each
with its `SourceLocation`, instead of failing or dropping silently.

### Scope

- A recoverable-diagnostic type in `html`'s domain, located, typed and `thiserror`-derived. It's distinct from the
  fatal `HtmlError` (decided below).
- The tokenizer and `TreeBuilder` push recoverable errors into the channel instead of returning `Err` or dropping
  silently.
- `parse` / `parse_with_sink` expose the collected diagnostics (e.g. `ParseOutcome { tree, diagnostics }`).
- `TokenSink` / `TreeSink` port contract update, a `html::PORT_SCHEMA_VERSION` bump and a PRD-008 migration note
  (ADR-0011 items 3–4). This partly addresses the known gap that `HtmlError` carries no source location.
- Tests: a malformed attribute name produces exactly one diagnostic at the right line and column, and the rest of the
  document parses.

### Non-goals

- Switching to the strict `AttributeName` or deleting `AttributeName::new_unchecked`. That's #28's job once this lands.
- Covering every WHATWG parse-error code. Only the errors the current tokenizer and tree builder can actually hit.
- A diagnostic cap or dedup (decision 7).
- Per-diagnostic logging or a DevTools surface in `alloy` — one summary line only.
- Relaxing `TagName` to WHATWG's permissive tag-name grammar (browsers create `<my_widget>`; we drop the tag).
- A DocumentType node or quirks mode. The doctype token is _consumed_ by design; only its `force_quirks` condition is
  reported (decision 13).
- Locations on character and comment tokens.
- A sink-side "strict mode" that aborts on diagnostics (`parse_error` is infallible — decision 11).

### Done

- No path in `html` drops input silently, and no recoverable condition aborts `parse`.
- `just gate` is green, and `alloy/tests/render_golden.rs` is byte-identical.

### Decisions (grill, 2026-10-05)

1. **Invalid tag names are recoverable.** One located diagnostic, the tag token is dropped, its content keeps parsing
   into the current parent. `TagName` stays strict.
2. **The attribute diagnostic comes from WHATWG, not `AttributeName`.** The tokenizer reports
   `unexpected-character-in-attribute-name` (`"` `'` `<`, §13.2.5.33) and _keeps_ the attribute.
3. **The channel lives in the port (html5ever model).** Tokenizer emits `Token::ParseError(ParseDiagnostic)`;
   `TreeBuilder` forwards to a new **required** `TreeSink::parse_error`. The sink owns the collection; `DomTreeSink`
   reports its own dom-side rejections the same way. Diagnostics are delivered in detection order, before the token
   that triggered them.
4. **Locations on boundary aggregates.** `AttributeEntry` carries its name's first-character `SourceLocation`;
   `TagToken` and `DoctypeToken` carry their `<`'s. Equality ignores location on all three.
5. **Type: `ParseDiagnostic { code: ParseErrorCode, location: SourceLocation }`**, `thiserror`-derived.
   `ParseErrorCode` is `#[non_exhaustive]`, one variant per WHATWG code. `HtmlError` stays fatal-only.
6. **Tokenizer codes in scope** — every one the current state machine can reach (attributes, tags, tag open, character
   references, input stream, comments/doctype).
7. **The diagnostic list is unbounded.**
8. **`parse` returns `Result<ParseOutcome { tree, diagnostics }, HtmlError>`.** `alloy` call sites emit one
   `tracing::debug!` summary and use `outcome.tree`.
9. **Remove unreachable `HtmlError` variants** `ParseError` and `UnexpectedEndOfInput`.
10. **Schema bump** `html::PORT_SCHEMA_VERSION` 1 → 2; contract record corrected; PRD-008 migration note.
11. **`TreeSink::parse_error` is infallible.**
12. **Tree-builder conditions in scope:** stray end tag; repeated `<html>`/`<body>` (reported, attributes merged via
    `add_attributes_if_missing`); misnesting; implied closures silent only when spec-implied (`p`/`li`); `DomTreeSink`
    rejecting an attribute.
13. **Doctype** consumed by design; a `force_quirks` doctype yields one diagnostic.
14. **`AttributeList` enforces uniqueness** (first wins).
15. **Manifest gate gets a third registry** (`SUPPORTED_PARSE_ERRORS` ↔ `## Parse errors`), one probe per code.

## Domain Concept Identification

### Existing Concepts (from codebase)

- `HtmlError`: the single fatal error of the port (ADR-0011 item 4); today also carries syntax variants nothing could
  recover from — relates to every `Result` in `TokenSink`/`TreeSink`.
- `SourceLocation`: line/column/offset value object; already produced by `Cursor::location()` — the location source
  for every diagnostic.
- `Token` / `TagToken` / `DoctypeToken`: the tokenizer→builder vocabulary; `Token` is `#[non_exhaustive]`.
- `AttributeList` / `AttributeEntry` / `AttributeName`: first-class attribute collection; today a `Vec` that accepts
  duplicates (last wins in `DomTreeSink`).
- `TokenSink` / `TreeSink` ports and `TreeBuilder`: the seam the diagnostics must cross. Implemented by `TreeBuilder`
  (`TokenSink`), `DomTreeSink`, `MockTreeSink`, the `&mut T` blanket, and a test sink in `tests/reentrancy_test.rs`.
- `Tokenizer` + `Cursor`: resumable state machine (`run`, `run_resumable`, `resume`), `pending_token` slot for the raw
  text end tag.
- Manifest gate (`MANIFEST.md` + `manifest_runner.rs`): two-way registry check for tags and syntax.

### New Concepts Required

- `ParseErrorCode`: closed vocabulary of recoverable conditions (WHATWG names where one exists) — governs what a
  diagnostic means.
- `ParseDiagnostic`: a coded, located recoverable error — the unit delivered through the port.
- `Diagnostics`: first-class ordered collection of diagnostics owned by a sink.
- `ParseOutcome`: the successful result of `parse` — tree plus diagnostics.
- `DuplicateAttribute`: the rejection value `AttributeList::insert` hands back.

### Key Business Rules

- A recoverable condition never aborts `parse`: it is reported and parsing continues (Done, decisions 3, 11).
- Nothing is dropped silently: every dropped, merged or rewritten construct has a diagnostic (Done, decisions 1, 6, 12,
  14).
- Diagnostics are ordered by detection and precede the token that triggered them (decision 3).
- Equality of tokens and attribute entries is location-independent (decision 4) — conformance mocks and tests compare
  structure, not position.
- Duplicate attribute: first wins; the later one is reported and dropped (decision 14).
- Spec-implied closures (`p`/`li` omission) are silent; any other implicit close is reported (decision 12).
- Fatal errors remain exactly: sink/adapter failure and value-object constructor validation (decision 9).

## Strategic Approach

### Solution Direction

- Add a domain `diagnostic` module; thread diagnostics through the existing token stream as `Token::ParseError`, so
  ordering relative to tree operations is automatic and suspension/resume needs no extra plumbing.
- The tree builder forwards them to a required, infallible `TreeSink::parse_error`; sinks own the collection
  (`DomTreeSink` exposes it through `ParseOutcome`; `MockTreeSink` records it as an event).
- The tokenizer replaces its `?`-on-validation paths with "report + recover" paths. Because that touches nearly every
  handler (five-to-seven `&mut` parameters each), group the per-tag scratch state into one struct so locations and
  reporting can be added without growing signatures further.
- Record locations where the data is born: the `<` of a tag, the first character of an attribute name.
- Document the pattern as ADR-0023 and update the contract record, PRD-008 and the manifest.

### Key Design Decisions

- **Diagnostics in-band (Token) vs. side collector** → in-band: one ordering, works across `run_resumable`/`resume`,
  mock-assertable; cost is a `Token` variant and a required `TreeSink` method (schema bump 1→2).
- **`parse_error` infallible vs. `Result`** → infallible: no sink can re-introduce an abort path (decision 11).
- **Uniqueness inside `AttributeList` vs. tokenizer check** → inside the collection: a list that cannot hold duplicates
  cannot drift back to last-wins in any sink (CLAUDE.md: first-class collections own their invariants).
- **New ADR vs. contract-record note** → ADR-0023: it reinterprets ADR-0011 item 4 ("exactly one error enum") and is a
  pattern css can reuse; CLAUDE.md requires an ADR for architectural decisions in the same PR.
- **Gate coverage**: `SUPPORTED_PARSE_ERRORS` ↔ manifest ↔ `ParseErrorCode::ALL`, so a code without a probe turns CI
  red.

### Alternatives Considered

- Parser-owned collector threaded outside the port: rejected — needs a second channel for `DomTreeSink` and partial
  hand-back on suspension.
- Reusing `HtmlError` for diagnostics: rejected — nothing in the type says abort vs. continue.
- Relaxing `TagName` to the WHATWG grammar: rejected — moves the failure into `dom::TagName::new` in the sink.
- Diagnostic cap: rejected by decision 7 (memory is linear in input, same order as the DOM).

## Risk & Gap Analysis

### Requirement Ambiguities

- The issue lists WHATWG codes by family, not exhaustively: the canvas fixes the final list as "every condition the
  current handlers can reach". Codes whose condition the tokenizer cannot distinguish today (e.g. no
  `eof-in-attribute-value` states) are out unless a handler can reach them.
- `unsupported-attribute-name` (a `dom`-rejected attribute) is alloy-specific, not WHATWG; it disappears when #28 makes
  the vocabularies identical.
- "Misnesting" is defined on the open-elements stack the builder actually keeps (it has no active-formatting list or
  adoption agency), so the diagnostic is approximate relative to a full WHATWG tree builder.

### Edge Cases

- `</` followed by EOF now emits text (`</`), changing the document for truncated inputs.
- An unfinished tag at EOF is dropped (spec) instead of emitted — the builder never sees it.
- A tag token dropped for an invalid name also drops its attributes and self-closing flag.
- `<p><span>a<div>`: the omission closure pops a non-implied `span` → reported; `<li><p>x</li>` stays silent.
- A diagnostic raised while a suspended script is pending must still be delivered in order after `resume`.
- Reconsumed characters must not double-report input-stream errors (NUL/control).
- Duplicate detection is case-insensitive because names are lowercased before comparison.

### Technical Risks

- **Blast radius**: five `TreeSink`/`TokenSink` implementors, `AttributeEntry::new`/`TagToken::new`/`DoctypeToken::new`
  call sites, `alloy` call sites — contained by compile errors, all inside `core/html` + `alloy`.
- **Unbounded diagnostics** on hostile input (`<a"""…>`): linear in input; accepted.
- **False positives** for valid entities outside the small entity table until #31.
- **Behaviour changes** (first-wins duplicates, EOF-in-tag drop, `</` text, repeated `body` attributes merged): the
  golden fixture `alloy/tests/fixtures/test_page.html` contains none of them (checked); other `alloy` tests that parse
  inline markup must be re-run.
- **Tooling**: `cargo-fuzz`/nightly and `cargo-llvm-cov`/`cargo-deny`/`arch-lint` may be absent in this sandbox — those
  gates cannot be proven locally and must be reported as not run.

### Acceptance Criteria Coverage

| AC# | Description                                                                          | Addressable? | Gaps/Notes                                                          |
| --- | ------------------------------------------------------------------------------------ | ------------ | ------------------------------------------------------------------- |
| 1   | No path in `html` drops input silently                                               | Yes          | Verified per code by manifest probes; doctype consumption documented |
| 2   | No recoverable condition aborts `parse`                                              | Yes          | Remaining `Err` = sink failure / constructor validation only        |
| 3   | `just gate` green                                                                    | Partial      | Tools missing in sandbox; run each available sub-gate, report rest  |
| 4   | `render_golden.rs` byte-identical                                                    | Yes          | Fixture hits no behaviour change                                    |
| 5   | Test: malformed attribute → exactly one diagnostic at right line/col, rest parses    | Yes          | `diagnostics_test.rs`                                               |
| 6   | Port contract update, `PORT_SCHEMA_VERSION` bump, PRD-008 migration note             | Yes          | Contract record's stale `= 2` corrected                             |
