# SPDD Analysis: CSS Parser, Specificity & Style Cascade (core/css)

## Original Business Requirement

### ROADMAP-IMPLEMENTACAO-V1: Fase F5 (Trilha B) & PRD-001 (§Aggregate Pipelines)

Implement the CSS subsystem in `core/css` transforming CSS stylesheets and a `DomTree` into a `StyledTree`:

- Tokenize and parse CSS syntax: selectors (tag, class, ID, descendant), property-value declarations, and rule blocks.
- Calculate CSS selector specificity (`(ids, classes, tags)`) and maintain a first-class collection `RuleSet`
  (ADR-0010).
- Implement selector matching against `DomTree` element nodes (tag name matching, class attribute matching, ID attribute
  matching, and recursive ancestor checking for descendant selectors).
- Implement style cascading and inheritance: match rules against nodes, sort declarations by specificity, resolve
  computed values (`Color`, `Px`, `DisplayType`), and inherit inheritable properties (e.g. `color`, `font-size`).
- Output the `StyledTree` aggregate combining DOM nodes with their resolved `ComputedStyle`, fulfilling the second link
  of the aggregate rendering pipeline: `HtmlStream -> DomTree (F4) -> StyledTree (F5) -> DisplayList (F7) -> PNG`.

---

## Domain Concept Identification

### Existing Concepts (from codebase)

- `DomTree`, `DomNode`, `NodeId`, `TagName`, `NodeData`, `AttributeMap`, `AttributeName` (`core/dom`).
- Aggregate Pipelines (ADR-0010, `CLAUDE.md`).

### New Concepts Required

- `Selector`: Pattern matching nodes in `DomTree`:
    - `Universal`: matches any element (`*`).
    - `Tag(TagName)`: matches element tag name (e.g. `h1`, `div`).
    - `Class(String)`: matches `.class-name`.
    - `Id(String)`: matches `#id-name`.
    - `Descendant(Box<Selector>, Box<Selector>)`: matches `ancestor descendant` (e.g. `div p`).
- `Specificity`: Tuple `(ids, classes, tags)` implementing `Ord` for deterministic cascade resolution.
- `PropertyName` & `PropertyValue`: Strongly typed CSS declaration parts.
- `Color`: Strongly typed 32-bit RGBA color representation (`Color::rgba(r, g, b, a)` or hex/named parser).
- `Px`: Strongly typed pixel unit value object (`Px(f32)`).
- `Declaration`: Pair of `PropertyName` and `PropertyValue`.
- `DeclarationList`: First-class collection wrapping declarations for a rule block.
- `Rule`: Pair of selectors and declarations.
- `RuleSet`: First-class collection of CSS rules (ADR-0010).
- `StyleSheet`: Root container of parsed CSS rules.
- `ComputedStyle`: Resolved style for a DOM node (display, color, background color, font size, margins, paddings, width,
  height).
- `StyledNode`: Entity binding a `NodeId` to its `ComputedStyle` and recursive styled children.
- `StyledTree`: Aggregate root representing the styled DOM hierarchy.
- `CssError`: Typed error enum for malformed selectors, values, or syntax.

### Key Business Rules

- **Specificity Precedence**: When multiple rules match a single node, properties from rules with higher specificity
  override lower specificity rules.
- **Rule Order (Source Order)**: If specificities are equal, later rules in the stylesheet override earlier rules.
- **Property Inheritance**: Inheritable properties (e.g. `color`, `font-size`) cascade from parent `ComputedStyle` to
  children if not explicitly specified.
- **Initial / Default Values**: When neither matched nor inherited, properties take standard CSS initial defaults (e.g.
  `display: inline` or `block` based on tag, `background-color: transparent`, `color: black`).

---

## Strategic Approach

### Solution Direction

- Implement Clean Architecture in `core/css`:
    - `src/domain/`: `selector.rs`, `specificity.rs`, `property.rs`, `declaration.rs`, `rule.rs`, `stylesheet.rs`,
      `computed.rs`, `styled_node.rs`, `error.rs`.
    - `src/application/`: `parser.rs` (`CssParser`), `cascade.rs` (`StyleCascade`).
    - `src/lib.rs`: Public facade.
- Add `dom = { workspace = true }` and `engine = { workspace = true }` to `core/css/Cargo.toml`.

### Key Design Decisions

- **Strong Types for CSS Primitives**: Zero raw `f32` or raw `u32` in public APIs; use `Px(f32)` and `Color(u32)` as
  prescribed by ADR-0010.
- **Zero Panic Parser**: CSS syntax errors (e.g. unknown properties or malformed selectors) are gracefully ignored or
  skipped according to CSS forward-compatible parsing rules.

---

## Acceptance Criteria Coverage

| AC#         | Descrição                                                                      | Endereçável nesta Fase (F5)? | Notas                                |
| :---------- | :----------------------------------------------------------------------------- | :--------------------------- | :----------------------------------- |
| F5 Pipeline | Parser CSS + Seletores (tag, classe, ID, descendente) + Cascata ➔ `StyledTree` | Sim                          | Entregável central da Fase F5.       |
| Invariantes | Especificidade calculada e herança aplicada na `StyledTree`                    | Sim                          | Suíte de testes em `css_cascade.rs`. |
