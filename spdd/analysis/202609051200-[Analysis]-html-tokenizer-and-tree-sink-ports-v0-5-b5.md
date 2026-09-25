# SPDD Analysis: HTML5 Tokenizer and Tree Sink Ports (v0.5 B5)

## Original Business Requirement

Realize Phase B5 of the Alloy v0.5 roadmap (`PRD-008`, `docs/v0-5-handoff/02-b5-html-tokenizer.md`), replacing the
`core/html` stub with an industrial-grade HTML5 tokenizer and tree-construction pipeline over `core/dom`:

1. **Replaceable Subsystem Port (`ADR-0011`)**: Provide `TokenSink` and `TreeSink` traits that allow alternative parser
   engines and alternative tree targets to be plugged into the Alloy skeleton.
2. **WHATWG §13.2.5 Tokenizer**: Implement streaming state-machine tokenization supporting the states required by real
   web documents (data, tag open, tag name, attributes, comments, doctype, and rawtext for `<script>`/`<style>`), with
   deterministic entity resolution (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&#...;`).
3. **Decoupled Domain Vocabulary (`ADR-0010`)**: Eliminate primitive obsession and foreign type leakage across the port
   boundary by introducing first-class domain value objects (`NodeHandle`, `TagName`, `Text`, `AttributeList`,
   `SourceLocation`).
4. **Resumable Parsing & Script Suspension (`PRD-008` §3)**: Enable the tokenizer to pause on `<script>` tokens, allow
   host-driven execution, and resume parsing with injected source fragments (supporting `document.write`).
5. **Independent Feature Isolation**: Make `core/dom` an optional cargo feature (`default = ["dom"]`) such that
   `core/html` can compile with `--no-default-features` using pure in-memory mock adapters.
6. **Deterministic Verification**: Ensure two-way consistency via `core/html/tests/data/MANIFEST.md`, conformance
   testing against `run_html_conformance`, fuzz testing under hostile bytes, and line coverage exceeding 85%.

## Domain Concept Identification

### Existing Concepts (from codebase)

- `dom::DomTree`: Arena-based DOM hierarchy storage (`core/dom/src/domain/tree.rs`). Owns nodes, elements, text, and
  mutation invariants.
- `dom::NodeId`: Arena index identifier (`core/dom/src/domain/node_id.rs`). Leaked in prior iterations; must be mapped
  strictly within the infrastructure adapter layer (`DomTreeSink`).
- `dom::DomError`: Mutation failure errors (`core/dom/src/domain/error.rs`). Converted into domain-specific tree
  construction errors.

### New Concepts Required

- `SourceLocation`: 1-indexed `line`, 1-indexed `column`, and 0-indexed `byte_offset` value object providing exact
  source provenance for errors and debug diagnostics.
- `NodeHandle`: Canonical opaque handle wrapping a `u32` identifier (`NodeHandle::root()` = 0). Protects port boundaries
  from leaking underlying arena or tree structures.
- `TagName`: Validated, lowercase normalized HTML tag identifier.
- `Text`: Immutable text string value object replacing naked primitives in text and comment tokens.
- `AttributeName`, `AttributeValue`, `AttributeEntry`, `AttributeList`: Strongly typed attribute collection maintaining
  case-folding, deduplication, and lookup without abbreviation anti-patterns.
- `Token`: Tag, character, comment, doctype, and EOF streaming tokens emitted by the tokenizer.
- `TokenSinkResult`: Flow-control signals (`Continue`, `SwitchToRawText`, `Suspend`, `Script`) mediating the handshake
  between tokenizer and tree builder.
- `HtmlError`: Unified domain error enum carrying `SourceLocation` on all syntax and parsing variants, implementing
  `Display` and `std::error::Error` without external derive macros.

### Key Business Rules

- **Hostile Input Safety**: Network bytes are adversarial; the tokenizer and tree builder must never panic or enter
  infinite loops on malformed inputs.
- **Port Inward Dependency**: The domain layer (`core/html/src/domain/`) and application port layer
  (`core/html/src/application/ports.rs`) must remain 100% free of dependencies on `core/dom`, `core/engine`, or any
  runtime crate.
- **Adapter Decoupling**: Sinks must allocate and maintain their own node handle table; caller interactions with
  `TreeSink` are purely handle-based.
- **Single-Line & Calisthenic Standards**: Functions must adhere to a single level of indentation, no naked primitives,
  no `else` keywords, and strict intention-revealing names without abbreviations.

## Strategic Approach

### Solution Direction

- **Hexagonal Architecture (`ADR-0010`, `ADR-0011`)**:
    - `src/domain/`: Pure value objects (`SourceLocation`, `NodeHandle`, `TagName`, `Text`, `Attribute*`, `HtmlError`,
      `Token`). Zero dependencies, zero I/O.
    - `src/application/`: Ports (`TokenSink`, `TreeSink`) and the conformance assertion suite (`run_html_conformance`).
    - `src/infrastructure/`: Tokenizer modular state machine, tree builder, `DomTreeSink` adapter, and `MockTreeSink`
      reference mock.
- **State Machine Modularization**: Break the tokenizer into granular sub-modules (`cursor`, `state`, `tag_state`,
  `attribute_state`, `doctype`, `rawtext`, `entity`) to satisfy the ~100-line entity guideline and preserve readability.
- **Script Suspension & Re-entrancy Protocol**: Provide `run_resumable` and `resume(injected_input)` methods on
  `Tokenizer`, preserving pending tokens and cursor state.

### Key Design Decisions

- **Domain-Owned NodeHandle vs. Foreign NodeId**: Retain `NodeHandle(u32)` at the port boundary. This prevents tight
  coupling to `core/dom` and allows testing against alternative tree sinks (including headless mocks and future Wasm/JS
  DOM bridges).
- **Decoupled Error Type with Hand-Written Traits**: Hand-write `Display` and `std::error::Error` for `HtmlError`
  instead of taking `thiserror` in `domain/`. Keeps the core domain zero-dependency and fast to compile.
- **Optional `dom` Feature Gate**: Expose `dom` as a default cargo feature. When disabled (`--no-default-features`),
  exclude `dom_sink.rs` while keeping the full tokenizer, tree builder, and mock sink fully operational.

### Alternatives Considered

- _Direct dom::NodeId Port Integration_: Rejected due to violation of ADR-0011 item 2 (port traits must speak domain
  value objects, not adapter types).
- _Monolithic Tokenizer File_: Rejected due to violation of Object Calisthenics entity size limit (original file was 655
  lines, far exceeding the 100-150 line guideline).

## Risk & Gap Analysis

### Requirement Ambiguities

- _Tag Omission Boundary_: Full WHATWG tag omission requires hundreds of transition rules. Resolved by bounding B5 scope
  to the minimal tags required by real documents (`<p>`, `<li>`, `<head>`, `<body>`, `<html>`).
- _Entity Coverage_: Named entities in HTML5 number over 2,000. Resolved by implementing standard XML/HTML core entities
  (`amp`, `lt`, `gt`, `quot`, `apos`, `nbsp`) plus hexadecimal and decimal numeric character references.

### Edge Cases

- _Malformed Less-Than (`<3`, `<<`, `<space`)_: The tokenizer must not discard characters following an unexpected
  delimiter. Emits character token and reconsumes following bytes correctly.
- _Prefixed Rawtext Closing Tags (`</scripture>`)_: Must not terminate a `<script>` block. Verified by exact string
  matching against the active tag name.
- _Re-entrancy Input Precedence_: Injected bytes from `document.write` must be processed before pending remaining bytes
  from the initial stream.

### Technical Risks

- _Hostile Byte Denial of Service_: Addressed via libFuzzer fuzz target (`fuzz/fuzz_targets/html_parse.rs`) and strict
  error bounds.
- _Clippy & Linter Violations_: Addressed via full workspace `-D warnings` enforcement and `arch-lint` architectural
  validation.

### Acceptance Criteria Coverage

| AC# | Description                                                     | Addressable? | Gaps/Notes                                    |
| --- | --------------------------------------------------------------- | ------------ | --------------------------------------------- |
| 1   | Replaceable `TokenSink` / `TreeSink` ports under ADR-0011       | Yes          | 100% compliant; contract record updated       |
| 2   | WHATWG §13.2.5 streaming tokenizer with state machine           | Yes          | Modularized into 8 sub-modules                |
| 3   | Source location tracking on all errors                          | Yes          | `SourceLocation` carried by all syntax errors |
| 4   | Re-entrancy and suspension for `<script>` and `document.write`  | Yes          | Tested via `tests/reentrancy_test.rs`         |
| 5   | Optional `dom` cargo feature and clean `--no-default-features`  | Yes          | Verified by CI `layering` job                 |
| 6   | Adapter conformance suite passing both real and mock sinks      | Yes          | Verified by `tests/conformance_test.rs`       |
| 7   | Strict Object Calisthenics, zero `unwrap`, 85%+ domain coverage | Yes          | 93.67% domain line coverage achieved          |
