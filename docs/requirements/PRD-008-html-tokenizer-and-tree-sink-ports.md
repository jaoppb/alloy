# PRD-008: HTML Tokenizer and Tree Sink Ports

- **Status**: Proposed
- **Author**: Core Architecture Team
- **Date**: 2026-08-28
- **Target Release**: v0.3

---

## 1. Executive Summary

`core/html` exposes two seams — `TokenSink` and `TreeSink` — so tree-construction policy can be replaced by implementing
`TreeSink` differently, and the tokenizer itself can be replaced behind a frozen token contract, without modifying
`core/dom` or any consumer. The `<script>` re-entrancy handshake (suspend/resume) is part of the contract, not an
afterthought. This PRD conforms to the Replaceable Port Contract of `ADR-0011` and supports the goal of `PRD-001:62`
("swapping the HTML parser").

---

## 2. Problem Statement

1. A monolithic parser cannot be forked incrementally: a fork that only wants different foster-parenting or different
   error recovery has to copy the whole crate.
2. Synchronous `<script>` blocks the tokenizer and `document.write` re-enters it. The immutable aggregate pipeline of
   `ADR-0010:114` does not model reentrancy, so the seam must expose an explicit suspend/resume point — and this must
   exist from the first tokenizer implementation (phase `F5`), not be retrofitted in `F10`.
3. Network input is hostile by definition; a panic in the parser is a denial of service (`roadmap §5`).

---

## 3. Architecture & Port Specifications

### 3.1 `Token` contract

```rust
#[non_exhaustive]
pub enum Token {
    Doctype(DoctypeToken),
    StartTag(TagToken),
    EndTag(TagToken),
    Character(Text),
    Comment(Text),
    Eof,
}
```

Input to the tokenizer is a decoded `&str`; character-encoding detection is an explicit upstream step and is out of
scope for this port. `Token` is frozen at integration point `I3`.

### 3.2 `TokenSink` trait (`html/application/ports.rs`)

```rust
pub trait TokenSink {
    fn process_token(&mut self, token: Token) -> TokenSinkResult;
}

pub enum TokenSinkResult {
    Continue,
    Script(ScriptHandle),
    SwitchTo(RawKind), // RawText, RcData, Plaintext
}
```

### 3.3 `TreeSink` trait (`html/application/ports.rs`, implemented by `core/dom`)

The replaceable tree-construction policy:

```rust
pub trait TreeSink {
    type Handle: Clone;

    fn create_element(&mut self, name: QualifiedName, attrs: Vec<Attribute>) -> Self::Handle;
    fn append(&mut self, parent: &Self::Handle, child: NodeOrText<Self::Handle>);
    fn append_before_sibling(&mut self, sibling: &Self::Handle, new_node: NodeOrText<Self::Handle>);
    fn add_attrs_if_missing(&mut self, target: &Self::Handle, attrs: Vec<Attribute>);
    fn remove_from_parent(&mut self, target: &Self::Handle);
    fn reparent_children(&mut self, from: &Self::Handle, to: &Self::Handle);
}
```

### 3.4 Suspend / resume handshake

`Tokenizer::run(input, sink)` returns `Run::Suspended { resume_at }` when a `<script>` start tag is closed. The host
runs the script (which may call `document.write` and push more input), then calls
`Tokenizer::resume(resume_at, extra_input)`. The sequence is deterministic: the same input and the same script output
produce the same token stream.

### 3.5 Reference implementation

The built-in HTML5 tokenizer and tree builder are adapters behind `TokenSink` and `TreeSink`; the default parse path
goes through the same traits an alternative implementation would.

---

## 4. Requirements & Invariants

1. **No foreign types** in the `Token` contract or the `TreeSink` signatures beyond `core/html`'s own value objects.
2. **Suspendable from day one**: the tokenizer is a resumable state machine in its first implementation (`F5`), per the
   ordering rule that `F7` precedes `F10`.
3. **Conformance**: the built-in adapters pass the declared html5lib subset (`roadmap §5`); "almost parses" is not
   acceptable.
4. **No panics on hostile input**: `cargo-fuzz` targets for the tokenizer and tree builder report zero panics in ten
   minutes per target (`roadmap §5`).
5. **Contract compliance**: this port satisfies all seven items of `ADR-0011`, including the `no-default-tree` feature
   (tokenizer testable with a stub `TreeSink`) and the `html-conformance` target.

---

## 5. Acceptance Criteria

- [ ] `Token`, `TokenSink`, `TreeSink`, and `TokenSinkResult` defined in `core/html` / `core/dom`, frozen at integration
      point `I3`.
- [ ] Built-in tokenizer and tree builder pass the declared html5lib subset.
- [ ] An alternative `TreeSink` mock builds a different in-memory structure from the same token stream **without
      changing** `core/html`.
- [ ] A `<script>` that calls `document.write("<p>x")` suspends the tokenizer, injects input on resume, and the final
      tree contains the written node.
- [ ] `core/html` tokenizer builds and tests with a stub `TreeSink` (feature `no-default-tree`).
- [ ] `cargo-fuzz` on the tokenizer and tree-builder targets: zero panics in ten minutes each.
