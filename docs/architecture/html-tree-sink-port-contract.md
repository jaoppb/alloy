# `TokenSink` / `TreeSink` port — ADR-0011 contract record

The `TokenSink` / `TreeSink` seam in `core/html` is a **Replaceable Subsystem Port** under `ADR-0011`. This document is
its contract record: the state of all seven mandatory items as of v0.5 B5.

| Item | Contract requirement                                                      | State                                                                                                                                                                                                                                                                            |
| ---- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | Seam PRD with variation + threat model                                    | ✅ `PRD-008` (variation model §1; threat model §2.3: network input is hostile by definition, a parser panic is a denial of service)                                                                                                                                              |
| 2    | Port traits: assoc types only, no adapter types, object-safe or companion | ✅ `TokenSink` and `TreeSink` are object-safe traits speaking only domain value objects (`NodeHandle`, `TagName`, `Text`, `AttributeList`). Zero foreign adapter types leak through the boundary. See §2 below                                                                   |
| 3    | Boundary aggregates: domain-owned, `#[non_exhaustive]`, schema version    | ✅ `Token`, `AttributeList`/`AttributeEntry`, `TagName`, `Text`, `NodeHandle`, `SourceLocation`, `HtmlError` domain-owned in `core/html`, `#[non_exhaustive]`; `html::PORT_SCHEMA_VERSION = 4` (see §3; `2` = recoverable diagnostics #36, `3` = vocabulary owned by `html` #28, `4` = full named character reference table #31) |
| 4    | Exactly one typed error, source location                                  | ✅ `HtmlError` is a single typed error enum carrying `SourceLocation` (`line`, `column`, `byte_offset`) on all syntax and parsing variants; derives `thiserror::Error` (ADR-0015). See §4                                                                                        |
| 5    | Written lifecycle & concurrency contract                                  | ✅ Written in §5 below; includes streaming tokenizer re-entrancy and suspension protocol for `<script>` and `document.write`                                                                                                                                                     |
| 6    | Conformance suite + reference adapter + `no-<adapter>`                    | ✅ `run_html_conformance` conformance suite; `DomTreeSink` (real, in `core/dom`) and `MockTreeSink` (reference mock) both pass; `core/html` has no `dom` dependency, enforced by `arch-lint`. See §6                                                                             |
| 7    | Frozen-API milestone                                                      | 🟡 Working surface at `html::PORT_SCHEMA_VERSION = 4`; freezes at integration point `I4`                                                                                                                                                                                         |

---

## 2. Object-safety and port decoupling (item 2)

Neither trait requires a `dyn`-dispatch companion because both are object-safe:

- `TokenSink::process_token(&mut self, token: Token) -> Result<TokenSinkResult, HtmlError>`
- `TokenSink::finish(&mut self) -> Result<(), HtmlError>`
- `TreeSink::create_element(&mut self, tag: TagName, attributes: &AttributeList) -> Result<NodeHandle, HtmlError>`
- `TreeSink::create_text(&mut self, text: &Text) -> Result<NodeHandle, HtmlError>`
- `TreeSink::create_comment(&mut self, text: &Text) -> Result<NodeHandle, HtmlError>`
- `TreeSink::append_child(&mut self, parent: NodeHandle, child: NodeHandle) -> Result<(), HtmlError>`
- `TreeSink::append_before_sibling(&mut self, sibling: NodeHandle, child: NodeHandle) -> Result<(), HtmlError>`
- `TreeSink::add_attributes_if_missing(&mut self, target: NodeHandle, attributes: &AttributeList) -> Result<(), HtmlError>`
- `TreeSink::remove_from_parent(&mut self, target: NodeHandle) -> Result<(), HtmlError>`
- `TreeSink::reparent_children(&mut self, from: NodeHandle, to: NodeHandle) -> Result<(), HtmlError>`
- `TreeSink::parse_error(&mut self, diagnostic: ParseDiagnostic)` — infallible, recoverable diagnostics (`ADR-0023`)
- `TreeSink::root_node(&self) -> NodeHandle`

Every method signature is decoupled from `dom::NodeId` using the domain-level newtype `NodeHandle(u32)`. Sinks map
handles to their internal node representations, ensuring `core/html` does not couple its public port traits to any
specific DOM tree crate.

---

## 3. Boundary aggregates and schema versioning (item 3)

Boundary types are domain-owned in `core/html` and marked `#[non_exhaustive]`:

- `NodeHandle`: canonical opaque handle wrapping a `u32` identifier (`NodeHandle::root()` = 0).
- `TagName`: validated, normalized lowercase tag identifier.
- `Text`: newtype wrapping character slice and text payload.
- `AttributeList`, `AttributeEntry`, `AttributeName`, `AttributeValue`: first-class collections preventing naked
  primitives and abbreviation anti-patterns.
- `SourceLocation`: immutable struct tracking 1-indexed `line`, 1-indexed `column`, and 0-indexed `byte_offset`.
- `Token` and `TagToken`: token stream representations emitted by the tokenizer. `TagToken`, `DoctypeToken` and
  `AttributeEntry` carry the `SourceLocation` they started at (ignored by equality); `Token::ParseError` carries a
  `ParseDiagnostic`.
- `AttributeList`: unique by name (`insert` keeps the first, hands the later one back as `DuplicateAttribute`).
- `ParseDiagnostic` / `ParseErrorCode` / `Diagnostics`: recoverable, located parse errors (`ADR-0023`). `ParseOutcome`
  (defined in `core/dom`, with `DomTreeSink` and `dom::parse`) is what parsing into a `DomTree` returns: the tree plus
  the diagnostics.
- `PORT_SCHEMA_VERSION`:

```rust
pub const PORT_SCHEMA_VERSION: u32 = 4; // 1 = B5 surface; 2 = recoverable diagnostics (#36); 3 = vocabulary owned by html (#28); 4 = full named character references (#31)
```

---

## 4. Single typed error with source location (item 4)

`HtmlError` is the single **fatal** error enum for the crate (`ADR-0011` item 4, `ADR-0023`):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HtmlError {
	InvalidTag { name: String, location: SourceLocation },
	InvalidAttribute { name: String, location: SourceLocation },
	TreeConstruction { message: String },
}
```

It is returned only for sink/adapter failure and value-object constructor validation; the tokenizer and tree builder
never abort on malformed input. In accordance with `ADR-0015`, it derives `thiserror::Error`. `core/dom` implements
`From<DomError> for HtmlError` (mapping tree invariant violations into `HtmlError::TreeConstruction`); `html` itself
does not know `dom`.

**Recoverable** malformations are `ParseDiagnostic { code: ParseErrorCode, location: SourceLocation }` (`thiserror`),
delivered in-band as `Token::ParseError` and to `TreeSink::parse_error` ahead of the token that triggered them. Codes
use the WHATWG names where one exists (`duplicate-attribute`, `eof-in-tag`, `unexpected-character-in-attribute-name`, …)
plus tree-builder codes (`stray-end-tag`, `end-tag-does-not-match-current-node`, `element-closed-implicitly`,
`unexpected-{html,body,head}-start-tag`, `quirks-mode-doctype`) and `invalid-attribute-name` (an attribute name breaking
the strict `AttributeName` rule is dropped and reported; ADR-0024). `dom::parse` returns them as
`ParseOutcome { tree, diagnostics }`. This closes the known gap that a parse error carried no source location.

Boundaries: the doctype is consumed by design (no `DocumentType` node); a quirks-forcing doctype is reported. The
diagnostic list is unbounded (linear in input). The set of codes is the manifest's `## Parse errors` table, checked in
both directions with an exact-location probe per code.

---

## 5. Lifecycle and concurrency contract (item 5)

### 5.1 Ownership of durable state

Rust owns all durable state (Skeleton and Muscle, `ADR-0003`). `TreeSink` implementations accumulate tree state across
token processing calls.

### 5.2 Threading model

`TokenSink` and `TreeSink` require `Send + Sync`. Tokenization drives a sink sequentially on a single thread.

### 5.3 Purity and determinism

`Tokenizer::run` is a pure function of its input text and the target sink. Given identical input, it produces identical
token streams and identical tree operations.

### 5.4 Re-entrancy, suspension, and script execution

The port protocol fully supports parser re-entrancy and suspension:

1. When the tree builder encounters an inline or blocking script, it emits `TokenSinkResult::Suspend` or
   `TokenSinkResult::Script(ScriptDescriptor)`.
2. The tokenizer halts parsing and preserves its cursor position, pending tokens, and tokenizer state.
3. The host can execute scripts or call `Tokenizer::resume` with injected content (e.g. `document.write`).
4. Injected source is processed before subsequent source bytes.

### 5.5 Resource ceilings and fault behaviour

- All operations return `Result<_, HtmlError>` and never panic on hostile or malformed byte sequences. Malformed input
  is not an error: it produces diagnostics (§4) and parsing continues.
- Diagnostics raised before a suspension are delivered before it, and those after `resume` after it, in report order.
- Tag and token length limits are validated before allocation.

---

## 6. Conformance suite, reference adapters and feature isolation (item 6)

- `html::run_html_conformance(sink: &mut dyn TreeSink)`: exhaustive validation suite testing element creation,
  hierarchical appends, text/comment insertion, sibling insertions, and reparenting invariants.
- `DomTreeSink`: concrete adapter in `core/dom` (`infrastructure/html_sink.rs`) mapping `NodeHandle` to `DomTree` via
  `NodeId`; its conformance run is `core/dom/tests/html_sink_conformance.rs`.
- `MockTreeSink`: reference mock adapter recording parser events (including `MockEvent::ParseError`) in memory without
  depending on `core/dom`.
- `core/html/tests/data/MANIFEST.md` lists tags, syntax and parse-error codes; `manifest_runner.rs` checks each registry
  against it in both directions and probes every code at its exact line and column.
- `core/html` is a leaf crate: the dependency is `dom → html` (ADR-0024), forbidden in the other direction by the
  `arch-lint` rules `html` (`deny-scope-dep`) and `html-isolated` (`restrict-use`). There is no `dom` feature.

---

## 7. Audit

Run tests and conformance checks:

```bash
cargo test -p html
cargo test -p dom --test html_sink_conformance
cargo llvm-cov --package html --ignore-filename-regex '(/application/|/infrastructure/)' --fail-under-lines 85
```
