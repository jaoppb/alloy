# `TokenSink` / `TreeSink` port — ADR-0011 contract record

The `TokenSink` / `TreeSink` seam in `core/html` is a **Replaceable Subsystem Port** under `ADR-0011`. This document is
its contract record: the state of all seven mandatory items as of v0.5 Phase P, written after `B5` (the WHATWG §13.2.5
tokenizer + tree builder over `core/dom`) shipped.

| Item | Contract requirement                                                      | State                                                                                                                                                                                                                                                                                |
| ---- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1    | Seam PRD with variation + threat model                                    | ✅ `PRD-008` (variation model §1; threat model §2.3: network input is hostile by definition, a parser panic is a denial of service)                                                                                                                                                  |
| 2    | Port traits: assoc types only, no adapter types, object-safe or companion | ✅ `TokenSink` and `TreeSink` are object-safe from the start — no generic method, no associated type. No companion needed, same shape as `css::CascadeResolver`/`network::HttpTransport`. See §2 below                                                                               |
| 3    | Boundary aggregates: domain-owned, `#[non_exhaustive]`, schema version    | ✅ `Token`, `AttributeList`/`AttributeEntry`, `HtmlError` domain-owned in `core/html`, `#[non_exhaustive]`; `html::PORT_SCHEMA_VERSION = 1` — **introduced in this phase** (see §3, a gap this record closes)                                                                        |
| 4    | Exactly one typed error, source location                                  | 🟡 `HtmlError` is one `#[non_exhaustive]` `thiserror` enum, but **carries no source location** (no line/column) — a real gap against this item, found in v0.5 Phase P review. See §4                                                                                                 |
| 5    | Written lifecycle & concurrency contract                                  | ✅ §5 below                                                                                                                                                                                                                                                                          |
| 6    | Conformance suite + reference adapter + `no-<adapter>`                    | ✅ `run_html_conformance`; `DomTreeSink` (real) and `MockTreeSink` (reference) both pass it (`core/html/tests/conformance_test.rs`). No `--no-default-features` build exists yet — see §6                                                                                            |
| 7    | Frozen-API milestone                                                      | 🟡 Introduced in this phase, not yet through a full release cycle unchanged — treat `html::PORT_SCHEMA_VERSION = 1` as the working surface; a shape change before the next minor version still just bumps it, no migration note owed until `PRD-008` gets its own freeze point named |

---

## 2. Object-safety (item 2)

Neither trait needed a `dyn`-dispatch companion:

- `TokenSink::process_token(&mut self, token: Token) -> Result<TokenSinkResult, HtmlError>` /
  `TokenSink::finish(&mut self) -> Result<(), HtmlError>`
- `TreeSink::create_element/create_text/create_comment/append_child/root_node` — each speaking only `&str`,
  `AttributeList`, `dom::NodeId`, and `HtmlError`.

Every parameter and return type is a concrete, `#[non_exhaustive]` boundary aggregate (`Token`, `TokenSinkResult`,
`AttributeList`). `&mut dyn TokenSink` and `&mut dyn TreeSink` both compile and are exactly what
`core/html/src/application/conformance.rs`'s `run_html_conformance` takes.

---

## 3. Boundary aggregates and the schema-version gap this phase closes (item 3)

`Token` (`#[non_exhaustive]` — `Doctype`/`StartTag`/`EndTag`/… ), `AttributeList`/`AttributeEntry`, and `HtmlError` are
all domain-owned in `core/html` and `#[non_exhaustive]`. What was missing until this phase: **`core/html` shipped B5
with no `PORT_SCHEMA_VERSION` constant at all** — every other port crate (`engine`, `graphics`, `css`, `network`,
`window`) has had one since it first satisfied ADR-0011 item 3. Phase P adds it:

```rust
pub const PORT_SCHEMA_VERSION: u32 = 1;
```

(`core/html/src/lib.rs`), documented the same way every other port's constant is, and set to `1` to record the surface
B5 shipped — not `0`, so a `match` on the value never has to special-case "before this existed".

---

## 4. The `HtmlError` source-location gap (item 4)

`HtmlError` is one `#[non_exhaustive]` `thiserror` enum (`ParseError(String)`, `UnexpectedEof { state: &'static str }`,
`InvalidTag(String)`, `InvalidAttribute(String)`, `DomError(#[from] dom::DomError)`) — the "exactly one typed error"
half of item 4. It does **not** carry the "source location" half: no variant has a line or column, unlike `CssError`'s
`SourceSpan`, `EngineError::ScriptRuntime`'s `SourceLocation`, or `NetworkError`'s `ProtocolPhase`. `UnexpectedEof`'s
`state: &'static str` names _which tokenizer state_ failed, which is diagnostic but not a position in the input text.

This is a real, currently-open gap, not a design choice recorded elsewhere — it was found by reading the domain error
during this phase's audit, not fixed here: threading a line/column through the streaming tokenizer touches the hot parse
loop (`ADR-0010:131`'s `<10μs`-adjacent budget reasoning that governs `core/css`'s parser applies here too) and is real
engineering work, not a documentation fix. **Tracked for a future `PRD-008` amendment** (a `SourceLocation` value object
mirroring `css::SourceSpan`, added to `ParseError`/`InvalidTag`/`InvalidAttribute` additively — `Token`'s shape does not
need to change, since diagnostics attach to the _error_, not the token).

---

## 5. Lifecycle and concurrency contract (item 5)

### 5.1 Ownership of durable state

**The Skeleton (Rust) owns all durable state** (`ADR-0003`). `DomTreeSink` owns the `dom::DomTree` it is building — the
_entire_ purpose of a `TreeSink` is to accumulate that state across many `process_token`/`create_*`/`append_child` calls
for one parse, then hand it to the caller via `into_tree`. This is different from `CascadeResolver`/ `HttpTransport`
(stateless per call): a `TreeSink` is stateful **within one parse**, and is not required to be reusable across two
independent documents — `DomTreeSink::new()` is cheap, and a caller builds a fresh one per `html::parse` call
(`core/html/src/lib.rs`'s `parse`/`parse_with_sink`).

### 5.2 Threading model

- Both traits are `Send + Sync`, but neither is required to support concurrent use _during_ one parse: `Tokenizer::run`
  drives one `TreeSink` sequentially through one input, single-threaded. `Send + Sync` exists so a sink can be _moved_
  to whichever thread parses (e.g. a future subresource-stylesheet-fetch worker thread, `alloy`'s Phase I4
  `event_loop.rs`), not so two threads share one sink mid-parse.
- `Tokenizer`/`TreeBuilder` are themselves single-use, single-threaded drivers — there is no `&self` reuse story
  analogous to `CascadeResolver::resolve` being callable concurrently from many threads.

### 5.3 Purity and determinism

`Tokenizer::run` is a pure function of its input text and the `TreeSink` it drives: the same HTML string against a fresh
`DomTreeSink` produces a field-for-field identical `DomTree` every time (exercised by `core/html/tests/corpus_test.rs`
against `example.com`-class fixtures, and indirectly by every golden-image test in `alloy` that starts from
`html::parse`). No wall-clock time, randomness, or ambient state is read.

### 5.4 Re-entrancy and suspension

**Deferred, by design** (`PRD-008` §2 item 2): the `<script>`/`document.write` suspend/resume handshake this PRD
identifies as needing an explicit seam is **not yet built** — B5's tokenizer treats `<script>`/`<style>` as raw-text
modes (`RawKind`), consuming their content as opaque text, but does not yet pause tokenization to let a script mutate
the tree mid-parse. This is the one item `PRD-008` names as a first-tokenizer-implementation requirement that B5 did not
fully close; tracked the same way as item 4's gap, for a future amendment.

### 5.5 Cancellation

There is no cooperative cancellation. `Tokenizer::run` consumes its whole input to completion or returns a typed
`HtmlError`; there is no "stop parsing this document from another thread" API.

### 5.6 Resource ceilings and fault behaviour

- **Fault behaviour**: every `TreeSink`/`TokenSink` method returns `Result<_, HtmlError>`; a malformed document (an
  unexpected EOF mid-tag, an invalid tag/attribute name) is a typed error, never a panic. `HtmlError::DomError` maps a
  `core/dom` invariant violation through without `core/html` inventing a second error for the same failure.
- **No nesting-depth ceiling yet**: unlike `core/css`'s `MAX_NESTING_DEPTH`/`MAX_LAYOUT_DEPTH`, this crate has no
  documented cap on tag-nesting depth. A pathologically deep document (thousands of unclosed tags) is bounded only by
  `core/dom`'s own arena growth, not refused early. Not yet a proven issue (`corpus_test.rs` carries no such fixture),
  but worth noting alongside item 4's gap for whoever next hardens this crate against hostile input.

### 5.7 Memory ceilings

Not enforced — mirrors the same gap `RuntimeEngine` (v0.1) and `core/css` (v0.5 B4) both carry: a large document grows
the `DomTree` arbitrarily. Deferred the same way.

---

## 6. Conformance suite, reference adapters and `no-<adapter>` (item 6)

- `html::run_html_conformance(sink: &mut dyn TreeSink)` — panics on the first violated rule, naming it. Checks: the root
  node is `dom::NodeId::root()`, element/text/comment creation and `append_child` all succeed and are readable back.
- `core/html/tests/conformance_test.rs` runs it against **both** the real adapter (`DomTreeSink`) and the reference mock
  (`MockTreeSink`, `infrastructure/mock.rs`, which records every operation as a `MockEvent` instead of building a real
  tree) — matching the "real adapter + reference mock both pass" shape `css`/`network`/`window` all use.
- **Gap**: `core/html/Cargo.toml` declares no features (`default = []`, nothing else) — there is no
  `--no-default- features` build that proves a script-facing adapter can be swapped out, because none has been written
  yet (unlike `css`'s `builtin-adapters` feature, which at least exists as a switch for later). `cargo test -p html`
  today already builds only Rust adapters, which is the same "nothing to gate yet" state `css` was in before `PRD-007`
  §3.4's scriptable cascade landed — noted here so whoever adds a script-facing `TreeSink` also adds the feature gate.

---

## Audit

Re-run `cargo test -p html` (conformance is `tests/conformance_test.rs`; corpus is `tests/corpus_test.rs`; the two-way
manifest consistency gate is `tests/manifest_runner.rs`, held blocking by the `css-conformance` CI job since this
phase). Check `html::PORT_SCHEMA_VERSION` against the last recorded value here whenever `Token`/`AttributeList`/
`HtmlError`/a trait signature changes. Items 4 (source location) and the `<script>` suspend/resume half of item 5.4 are
the two open gaps this record tracks rather than closes — revisit both before `core/js` (v0.7) needs either.
