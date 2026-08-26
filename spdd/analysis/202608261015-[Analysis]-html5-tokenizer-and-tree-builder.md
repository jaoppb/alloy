# SPDD Analysis: HTML5 Tokenizer & Tree Construction (core/html)

## Original Business Requirement

### ROADMAP-IMPLEMENTACAO-V1: Fase F4 (Trilha B) & PRD-001 (§Aggregate Pipelines)

Implement the HTML processing pipeline in `core/html` transforming raw HTML text streams into a valid `DomTree`:

- Implement a robust HTML tokenizer emitting DOCTYPE, StartTag, EndTag, Character, Comment, and EOF tokens.
- Handle attribute parsing (quoted with single or double quotes, unquoted values).
- Support standard character entity decoding (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&#39;`).
- Implement the Tree Builder state machine maintaining an active element insertion stack (`open_elements: Vec<NodeId>`).
- Auto-close open elements when matching end tags arrive; handle void/self-closing elements (`img`, `br`, `hr`, `input`,
  `meta`, `link`).
- Output a completely constructed `DomTree` aggregate ready for CSS styling (F5) and layout rendering (F7).

---

## Domain Concept Identification

### Existing Concepts (from codebase)

- `DomTree` (`core/dom`): Aggregate root arena managing nodes.
- `NodeId` (`core/dom`): Strongly typed node handle.
- `TagName` (`core/dom`): Validated lowercase tag name.
- `AttributeMap`, `AttributeName`, `AttributeValue` (`core/dom`): First-class collection of element attributes.
- `DomService` (`core/dom`): Subtree serializer for debugging and validation.

### New Concepts Required

- `HtmlToken`: Discriminated union of HTML lexical tokens:
    - `Doctype(String)`
    - `StartTag { name: TagName, attributes: AttributeMap, self_closing: bool }`
    - `EndTag(TagName)`
    - `Character(String)`
    - `Comment(String)`
    - `Eof`
- `HtmlTokenizer`: Stateful lexer streaming `HtmlToken` from a UTF-8 character stream.
- `TreeBuilder`: Stateful parser managing an active stack of open `NodeId` elements and attaching nodes to the
  `DomTree`.
- `HtmlParser`: Application service orchestrating tokenization and tree construction.
- `HtmlError`: Domain error enum (`UnexpectedEof`, `MalformedTag`, `InvalidEntity`, `TreeError(DomError)`).

### Key Business Rules

- **Void Elements**: Void elements (e.g. `img`, `br`, `hr`, `input`, `meta`, `link`) must not push onto the open element
  stack; they are attached as immediate leaf elements without expecting an end tag.
- **Hierarchy Resiliency**: When an end tag `</tag>` is encountered, the insertion stack pops open elements up to the
  matching tag name.
- **Whitespace & Text Coalescing**: Consecutive text characters are accumulated into a single `Character` token to avoid
  creating dozens of tiny adjacent text nodes in the DOM arena.
- **Entity Replacement**: Standard named entities (`&amp;` ➔ `&`, `&lt;` ➔ `<`, `&gt;` ➔ `>`, `&quot;` ➔ `"`, `&#39;` ➔
  `'`) are decoded into their literal characters.

---

## Strategic Approach

### Solution Direction

- Implement Clean Architecture in `core/html`:
    - `src/domain/`: `token.rs` (`HtmlToken`, `HtmlError`), `tokenizer.rs` (`HtmlTokenizer`), `tree_builder.rs`
      (`TreeBuilder`), `entities.rs` (entity decoder).
    - `src/application/`: `parser.rs` (`HtmlParser`, `parse_html`).
    - `src/lib.rs`: Public facade exporting `parse_html`, `HtmlParser`, `HtmlToken`, `HtmlError`.
- Add `dom = { workspace = true }` and `engine = { workspace = true }` to `core/html/Cargo.toml`.

### Key Design Decisions

- **Arena Insertion Stack**: The `TreeBuilder` maintains `open_elements: Vec<NodeId>`. The current insertion point is
  always `open_elements.last()`.
- **Zero Copy / String Slices**: Tokenizer works over `&str` using character indices to minimize allocations.
- **Deterministic Root Guarantee**: If no root `<html>` or `document` is explicitly provided, `TreeBuilder`
  automatically creates the document root so every valid HTML parse produces a non-empty `DomTree`.

---

## Risk & Gap Analysis

### Requirement Ambiguities

- Handling ill-formed HTML (e.g. unclosed tags `<div><p>text</div>`): The tree builder should auto-close `<p>` when
  `</div>` is reached, mimicking standard browser resilience.

### Edge Cases

- Script and style tags: For v0.3 headless pipeline, `<script>` and `<style>` contents are parsed as text children of
  their respective element nodes.
- Consecutive text chunks: Coalesce into single text node.

### Acceptance Criteria Coverage

| AC#         | Descrição                                            | Endereçável nesta Fase (F4)? | Notas                                           |
| :---------- | :--------------------------------------------------- | :--------------------------- | :---------------------------------------------- |
| F4 Pipeline | Tokenizer HTML5 + Tree Builder produzindo `DomTree`  | Sim                          | Entregável central da Fase F4.                  |
| Invariantes | Aciclicidade e integridade da árvore DOM preservadas | Sim                          | Validado com `core/dom` e testes de round-trip. |
