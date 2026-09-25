# `TokenSink` / `TreeSink` port — ADR-0011 contract record

The `TokenSink` / `TreeSink` seam in `core/html` is a **Replaceable Subsystem Port** under `ADR-0011`. This document is
its contract record: the state of all seven mandatory items as of v0.5 B5.

| Item | Contract requirement                                                      | State                                                                                                                                                                                                          |
| ---- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | Seam PRD with variation + threat model                                    | ✅ `PRD-008` (variation model §1; threat model §2.3: network input is hostile by definition, a parser panic is a denial of service)                                                                            |
| 2    | Port traits: assoc types only, no adapter types, object-safe or companion | ✅ `TokenSink` and `TreeSink` are object-safe traits speaking only domain value objects (`NodeHandle`, `TagName`, `Text`, `AttributeList`). Zero foreign adapter types leak through the boundary. See §2 below |
| 3    | Boundary aggregates: domain-owned, `#[non_exhaustive]`, schema version    | ✅ `Token`, `AttributeList`/`AttributeEntry`, `TagName`, `Text`, `NodeHandle`, `SourceLocation`, `HtmlError` domain-owned in `core/html`, `#[non_exhaustive]`; `html::PORT_SCHEMA_VERSION = 2` (see §3)        |
| 4    | Exactly one typed error, source location                                  | ✅ `HtmlError` is a single typed error enum carrying `SourceLocation` (`line`, `column`, `byte_offset`) on all syntax and parsing variants; derives `thiserror::Error` (ADR-0015). See §4                      |
| 5    | Written lifecycle & concurrency contract                                  | ✅ Written in §5 below; includes streaming tokenizer re-entrancy and suspension protocol for `<script>` and `document.write`                                                                                   |
| 6    | Conformance suite + reference adapter + `no-<adapter>`                    | ✅ `run_html_conformance` conformance suite; `DomTreeSink` (real) and `MockTreeSink` (reference mock) both pass; `feature = "dom"` is optional with `--no-default-features` verified in CI. See §6             |
| 7    | Frozen-API milestone                                                      | 🟡 Working surface at `html::PORT_SCHEMA_VERSION = 2`; freezes at integration point `I4`                                                                                                                       |

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
- `Token` and `TagToken`: token stream representations emitted by the tokenizer.
- `PORT_SCHEMA_VERSION`:

```rust
pub const PORT_SCHEMA_VERSION: u32 = 2;
```

---

## 4. Single typed error with source location (item 4)

`HtmlError` is the single domain error enum for the crate:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HtmlError {
	ParseError { location: SourceLocation, message: String },
	UnexpectedEof { location: SourceLocation, state: &'static str },
	InvalidTag { location: SourceLocation, name: String },
	InvalidAttribute { location: SourceLocation, name: String },
	TreeConstruction { message: String },
}
```

Every tokenizer and parser syntax error captures its precise `SourceLocation`. In accordance with `ADR-0015`,
`HtmlError` derives `thiserror::Error` for typed diagnostic formatting without runtime cost. When the `dom` cargo
feature is enabled, `From<dom::DomError>` automatically maps tree invariant violations into
`HtmlError::TreeConstruction`.

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

- All operations return `Result<_, HtmlError>` and never panic on hostile or malformed byte sequences.
- Tag and token length limits are validated before allocation.

---

## 6. Conformance suite, reference adapters and feature isolation (item 6)

- `html::run_html_conformance(sink: &mut dyn TreeSink)`: exhaustive validation suite testing element creation,
  hierarchical appends, text/comment insertion, sibling insertions, and reparenting invariants.
- `DomTreeSink`: concrete adapter mapping `NodeHandle` to `dom::DomTree` via `NodeId`.
- `MockTreeSink`: reference mock adapter recording parser events in memory without depending on `core/dom`.
- `core/html/Cargo.toml` provides the optional `dom` feature (`default = ["dom"]`). When building with
  `--no-default-features`, `core/html` compiles completely decoupled from `core/dom`, verified by CI.

---

## 7. Audit

Run tests and conformance checks:

```bash
cargo test -p html
cargo test -p html --no-default-features
cargo llvm-cov --package html --ignore-filename-regex '(/application/|/infrastructure/)' --fail-under-lines 85
```
