# SPDD Analysis — v0.2 F3 (`core/dom`): the DOM tree aggregate

> Phase 0 artefact for the first half of v0.2 "DOM scriptável e contido". Builds on the delivered v0.1
> (`spdd/prompt/202608300900-[Feat]-rhai-runtime-v0-1-f2.md`). Consolidated from
> `docs/reports/IMPLEMENTACAO-DETALHADA-V0-2.md` §2.1–2.4 and §3 (F3 steps), `PRD-002:89`, `PRD-003`, `ADR-0003`,
> `ADR-0010` (Clean Architecture + the 9 Object Calisthenics rules), `ADR-0011` (Replaceable Port Contract), `ADR-0014`
> (Structured Logging with `tracing`), `ADR-0015` (Typed Errors with `thiserror`), and `docs/architecture/overview.md`
> §3, incorporating PR #5 code review resolutions.

## Original Business Requirement

From `ROADMAP-IMPLEMENTACAO-V1.md` §3.2 (F3), `docs/reports/IMPLEMENTACAO-DETALHADA-V0-2.md`, and PR #5 review comments:

> **F3 — `core/dom`**: `NodeId`, arena, `Children`, `TagName`, `AttributeMap`, invariantes, travessia, serializador.
> Entregável verificável: testes de aciclicidade, pai único, ordem de travessia. Esforço `[modelado]`: 12–18 d.
>
> Decisão 2.1: `core/dom` é crate de domínio puro. Não depende de `engine`, e portanto não puxa `rhai` transitivamente —
> mantém o portão "Domínio sem engine" (N-04, `PRD-001:99`) verde por construção e evita que `core/html`, `core/css` e
> `core/js` herdem o interpretador. As dependências e limites de camada são garantidos mecanicamente via `arch-lint`.
>
> Decisão 2.2: `DomTree` é o agregado: `Vec<Slot>` indexado por `NodeId(u32)`. `Slot` é `Occupied(NodeData)` ou
> `Tombstone`. Remoção deixa tombstone e **não reutiliza** o índice na v0.2; sem índice geracional (fica para v0.9 /
> C-13). `NodeData` = `kind: NodeKind`, `parent: Option<NodeId>`, `children: Children`. `NodeKind` = `Document` ·
> `Element(ElementData)` · `Text(TextContent)` · `Comment(CommentContent)`. Value objects: `TagName` (enum tipado forte
> com elementos HTML5 W3C padrão + `Custom(String)`, provendo `is_void()`), `TextContent`, `AttributeName`,
> `AttributeValue`; `Children(Vec<NodeId>)` e `AttributeMap(BTreeMap<AttributeName, AttributeValue>)` (ordem
> determinística e lookups rápidos) — nunca coleção padrão pública. `NodeId::root()` provê o id semântico da raiz
> `Document`.
>
> Invariantes, garantidas só por métodos de `DomTree` (sem campo público mutável): **(1)** aciclicidade — `append_child`
> recusa se `child ∈ ancestors(parent)` → `WouldCycle`; **(2)** pai único — anexar nó com pai o desanexa antes; **(3)**
> sem auto-pai → `SelfParent`; **(4)** `Document` é raiz única, não desanexável nem removível → `CannotDetachDocument`;
> **(5)** todo `NodeId` em um `Children` resolve para `Occupied` com `parent` de volta.
>
> Decisão 2.3: `descendants(root)` e `ancestors(node)` são iteradores com pilha `Vec<NodeId>` explícita — **sem
> recursão**. **Não** há exceção de Object Calisthenics para `core/dom` — as 9 regras valem inteiras.
>
> Decisão 2.4: `core/dom/src/domain/error.rs` define **um** enum (`DomError`), derivado com `thiserror` (ADR-0015).
> `core/dom` não conhece `EngineError`. O mapeamento `DomError → EngineError::Dom` é do adaptador (I1), nunca de
> `core/dom`.
>
> Serializador (F3 passo 5): `serialize_html(&DomTree, NodeId) -> Result<String, DomError>` puro e determinístico em
> `application/serialize.rs`; escape completo de entidades nomeadas W3C (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&nbsp;`,
> `&copy;`, `&euro;`, `&trade;`, etc.) — é a saída do micro-entregável.

## Domain Concept Identification

### Existing Concepts (from codebase)

- **`NodeId(u32)`** — the newtype `ADR-0010` rule 3 and `CLAUDE.md` write verbatim. Constructed only by `DomTree` with a
  semantic constructor `NodeId::root()` (`NodeId(0)`).
- **Object Calisthenics rules 1–9** (`ADR-0010:127-137`) — enforced across `core/engine`, `core/runtime/rhai`, and
  `core/dom`.
- **`EngineValue` / `EngineError`** — the boundary aggregates. `core/dom` does not name either; the seam is I1's
  adapter.
- **`#![forbid(unsafe_code)]`** — enforced across the crate.
- **`thiserror`** — derived typed errors for domain/adapter crates per ADR-0015.
- **`arch-lint`** — architectural boundary linter enforcing domain purity and layer isolation.

### New Concepts Required

- **`DomTree`** — the aggregate root. Owns `Vec<Slot>` and the `NodeId` of the single `Document`. All mutation and
  invariant enforcement is here.
- **`Slot`** — `Occupied(NodeData)` | `Tombstone`.
- **`NodeData`** — `{ kind: NodeKind, parent: Option<NodeId>, children: Children }`.
- **`NodeKind`** — `Document` | `Element(ElementData)` | `Text(TextContent)` | `Comment(CommentContent)`.
- **`ElementData`** — `{ tag: TagName, attributes: AttributeMap }`.
- **Strongly-Typed `TagName`** — enum with all standard W3C HTML5 elements (`Html`, `Head`, `Body`, `Div`, `Span`, `P`,
  `A`, `Img`, `Br`, `Input`, `Meta`, `Link`, `Hr`, etc.) and `Custom(String)` for autonomous custom elements. Embeds
  `is_void()` as a domain query method.
- **Value objects** — `TextContent` (any string), `CommentContent` (any string), `AttributeName` (non-empty, ASCII,
  lowercased, no forbidden punctuation), `AttributeValue` (any string).
- **First-class collections** — `Children(Vec<NodeId>)` (insertion order) and
  `AttributeMap(BTreeMap<AttributeName, AttributeValue>)` (deterministic sorted order, $O(\log N)$ get/set/remove). No
  public std collections.
- **`DomError`** — typed enum derived via `#[derive(thiserror::Error)]`.
- **Traversal iterators** — `Descendants<'_>` (pre-order, explicit stack) and `Ancestors<'_>` (walk to `Document`).
- **`serialize_html`** — `application/serialize.rs`, pure, deterministic, calling `element.tag().is_void()` and applying
  full named entity escaping.

### Key Business Rules

- **Acyclicity** — `append_child(parent, child)` fails `WouldCycle` when `parent == child` (→ `SelfParent`) or `child`
  is an ancestor.
- **Single parent** — attaching a node that already has a parent removes it from the old parent's `Children` first.
- **`Document` is the irremovable root** — `detach` / `remove` on `NodeId::root()` is `CannotDetachDocument`.
- **`Children` ⇄ `parent` coherence** — every id in `Children` resolves to an `Occupied` slot whose `parent` points
  back.
- **Void tag semantics** — void elements (`is_void() == true`) omit closing tags in serialization.
- **Determinism** — `serialize_html` of the same tree yields byte-identical output; attribute ordering is deterministic
  via `BTreeMap`.
- **Entity Escaping** — text and attribute escaping follows W3C HTML specification with full named entity lookup.

---

## Strategic Approach

### Solution Direction

A clean `domain/` and `application/` architecture:

- `core/dom/src/domain/` encapsulates all entities, invariants, and value objects with `#![forbid(unsafe_code)]`.
- `core/dom/src/application/serialize.rs` folds the DOM tree to HTML with full entity escaping and void-tag handling.
- `arch-lint.toml` enforces that `core/dom` never imports engine or runtime adapters.

### Key Design Decisions

- **`BTreeMap` for `AttributeMap`**: Fast lookups with deterministic alphabetical key sorting, avoiding randomized hash
  iteration.
- **Strongly-typed `TagName` enum**: Replaces string matching with W3C HTML5 element variants and `Custom(String)`,
  putting `is_void()` in the domain.
- **`thiserror` in `core/dom`**: Conforms to ADR-0015, eliminating manual error boilerplate while maintaining typed
  variants.
- **Full W3C named entity escaping**: Ensures standard HTML serialization compatibility for named symbols (`&nbsp;`,
  `&copy;`, `&euro;`, `&trade;`, etc.).
- **`NodeId::root()` constructor**: Explicit semantic factory method for the root document ID.

### Alternatives Considered

- _String wrapper `TagName` with helper regex/matches_: Rejected in review due to lack of type safety and brittle string
  matching.
- _`Vec` storage for `AttributeMap`_: Rejected in review due to linear search overhead.
- _`HashMap` storage for `AttributeMap`_: Rejected during `/grill-me` due to non-deterministic serialization order
  requiring per-call sorting.

---

## Risk & Gap Analysis

### Edge Cases

- Custom elements (`TagName::Custom`) containing valid alphanumeric characters and hyphens must parse successfully and
  return `is_void() == false`.
- Entity escaping must correctly transform both markup delimiters (`&`, `<`, `>`, `"`) and unicode symbols with named
  HTML entities without corrupting other unicode text.

### Technical Risks

- **Lint Gate**: Strict workspace clippy and `arch-lint` rules must pass with `thiserror` added to
  `core/dom/Cargo.toml`.

### Acceptance Criteria Coverage

| AC (source)                                      | Addressable? | Notes                                     |
| ------------------------------------------------ | ------------ | ----------------------------------------- |
| Acyclicity & Single Parent invariants            | Yes          | Verified by `tree_invariants.rs`          |
| `TagName` strongly-typed enum & domain `is_void` | Yes          | Verified by unit tests and `serialize.rs` |
| Deterministic `BTreeMap` attribute serialization | Yes          | Verified by `serialize.rs` tests          |
| Comprehensive HTML entity escaping               | Yes          | Verified by `serialize.rs` entity tests   |
| `thiserror` error formatting                     | Yes          | Verified by `DomError` display tests      |
| `arch-lint` boundary enforcement                 | Yes          | Verified by `arch-lint check`             |
