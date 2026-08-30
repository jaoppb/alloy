# SPDD Analysis — v0.2 F3 (`core/dom`): the DOM tree aggregate

> Phase 0 artefact for the first half of v0.2 "DOM scriptável e contido". Builds on the delivered v0.1
> (`spdd/prompt/202608300900-[Feat]-rhai-runtime-v0-1-f2.md`). Consolidated from
> `docs/reports/IMPLEMENTACAO-DETALHADA-V0-2.md` §2.1–2.4 and §3 (F3 steps), `PRD-002:89`, `PRD-003`, `ADR-0003`,
> `ADR-0010` (Clean Architecture + the 9 Object Calisthenics rules), `ADR-0011` (Replaceable Port Contract), and
> `docs/architecture/overview.md` §3.

## Original Business Requirement

From `ROADMAP-IMPLEMENTACAO-V1.md` §3.2 (F3) and `docs/reports/IMPLEMENTACAO-DETALHADA-V0-2.md`:

> **F3 — `core/dom`**: `NodeId`, arena, `Children`, `TagName`, `AttributeMap`, invariantes, travessia, serializador.
> Entregável verificável: testes de aciclicidade, pai único, ordem de travessia. Esforço `[modelado]`: 12–18 d.
>
> Decisão 2.1: `core/dom` é crate de domínio puro — **zero dependências**. Não depende de `engine`, e portanto não puxa
> `rhai` transitivamente — mantém o portão "Domínio sem engine" (N-04, `PRD-001:99`) verde por construção e evita que
> `core/html`, `core/css` e `core/js` herdem o interpretador. O registro de `DomNode` como tipo de engine (C-03) vive em
> `core/runtime/rhai/infrastructure/dom_bindings.rs` — trabalho de I1, não de F3.
>
> Decisão 2.2: `DomTree` é o agregado: `Vec<Slot>` indexado por `NodeId(u32)`. `Slot` é `Occupied(NodeData)` ou
> `Tombstone`. Remoção deixa tombstone e **não reutiliza** o índice na v0.2; sem índice geracional (fica para v0.9 /
> C-13). `NodeData` = `kind: NodeKind`, `parent: Option<NodeId>`, `children: Children`. `NodeKind` = `Document` ·
> `Element(ElementData)` · `Text(TextContent)` · `Comment(CommentContent)`. Value objects: `TagName` (valida não-vazio,
> ASCII, minúsculo na construção), `TextContent`, `AttributeName`, `AttributeValue`; `Children(Vec<NodeId>)` e
> `AttributeMap` (ordem de inserção preservada) — nunca coleção padrão pública.
>
> Invariantes, garantidas só por métodos de `DomTree` (sem campo público mutável): **(1)** aciclicidade — `append_child`
> recusa se `child ∈ ancestors(parent)` → `WouldCycle`; **(2)** pai único — anexar nó com pai o desanexa antes; **(3)**
> sem auto-pai → `SelfParent`; **(4)** `Document` é raiz única, não desanexável nem removível → `CannotDetachDocument`;
> **(5)** todo `NodeId` em um `Children` resolve para `Occupied` com `parent` de volta.
>
> Decisão 2.3: `descendants(root)` e `ancestors(node)` são iteradores com pilha `Vec<NodeId>` explícita — **sem
> recursão**. **Não** há exceção de Object Calisthenics para `core/dom` — as 9 regras valem inteiras.
>
> Decisão 2.4: `core/dom/src/domain/error.rs` define **um** enum (`DomError`). `core/dom` não conhece `EngineError`. O
> mapeamento `DomError → EngineError::Dom` é do adaptador (I1), nunca de `core/dom`.
>
> Serializador (F3 passo 5): `serialize_html(&DomTree, NodeId) -> String` puro e determinístico em
> `application/serialize.rs`; escape de `&<>` — é a "saída" do micro-entregável (um script Rhai constrói uma árvore DOM
> e a serializa na saída).

## Domain Concept Identification

### Existing Concepts (from codebase)

- **`NodeId(u32)`** — the newtype `ADR-0010` rule 3 and `CLAUDE.md` write verbatim. Does **not** exist yet; `core/dom`
  is still the `add()` stub. It is the arena index, `Copy`, constructed only by `DomTree`.
- **Object Calisthenics rules 1–9** (`ADR-0010:127-137`) — already enforced across `core/engine` / `core/runtime/rhai`
  (`spdd/prompt/…-f2.md` Norms). F3 inherits them with **no exception** (unlike the future tokenizer interior).
- **`EngineValue` / `EngineError`** — the v0.1 boundary aggregates. F3 **must not** name either; the seam between DOM
  and the engine is I1's adapter (`ADR-0011` item 3: conversion is an explicit mapping function, never a re-export).
- **`#![forbid(unsafe_code)]`** — every crate carries it; the arena's two-sequential-slot-mutation pattern must hold
  without `unsafe` and without simultaneous `&mut`.

### New Concepts Required

- **`DomTree`** — the aggregate root. Owns `Vec<Slot>` and the `NodeId` of the single `Document`. All mutation and all
  invariant enforcement is here (`ADR-0010` rule 8: no public mutable field; rule 9: state + behaviour bundled).
- **`Slot`** — `Occupied(NodeData)` | `Tombstone`. A removed node's slot becomes `Tombstone` and its index is never
  reissued in v0.2.
- **`NodeData`** — `{ kind: NodeKind, parent: Option<NodeId>, children: Children }`. Private fields; the tree
  reads/writes it through methods.
- **`NodeKind`** — `Document` | `Element(ElementData)` | `Text(TextContent)` | `Comment(CommentContent)`.
- **`ElementData`** — `{ tag: TagName, attributes: AttributeMap }`.
- **Value objects** — `TagName` (non-empty, ASCII alnum + `-`, lowercased on construction), `TextContent` (any string),
  `CommentContent` (any string), `AttributeName` (non-empty, ASCII, lowercased, no control/quote/`=`/`/`/`>` chars),
  `AttributeValue` (any string).
- **First-class collections** — `Children(Vec<NodeId>)` (insertion order; `push`, `remove`, `insert_before`, `position`,
  `contains`, `iter`); `AttributeMap(Vec<(AttributeName, AttributeValue)>)` (insertion order; `set` = update-in-place or
  append, `get`, `remove`, `iter`). No public `Vec`.
- **`DomError`** — one typed enum, `#[non_exhaustive]` (so I1's mapping stays forward-compatible).
- **Traversal iterators** — `Descendants<'_>` (pre-order / document order, explicit stack) and `Ancestors<'_>` (child →
  … → `Document`, explicit walk). Both `Iterator<Item = NodeId>`, both borrow `&DomTree`, neither recurses.
- **`serialize_html`** — `application/serialize.rs`, pure, deterministic; explicit work-stack, no recursion.

### Key Business Rules

- **Acyclicity** governs `DomTree` + `NodeData.parent` + `Children`: `append_child(parent, child)` fails `WouldCycle`
  when `parent == child` (→ `SelfParent`, checked first) or `child` is an ancestor of `parent`.
- **Single parent** governs `NodeData.parent` + every `Children`: attaching a node that already has a parent removes it
  from the old parent's `Children` first — one atomic move.
- **`Document` is the irremovable root** — `detach` / `remove` on the `Document` id is `CannotDetachDocument`. The tree
  is born with exactly one `Document` at a fixed id.
- **`Children` ⇄ `parent` coherence** — every id in a node's `Children` resolves to an `Occupied` slot whose `parent`
  points back. No method may leave a dangling child or a one-way link.
- **Character-data / element typing** — `tag` / attribute operations require `NodeKind::Element`; `text` operations
  require `NodeKind::Text`; only `Document` and `Element` may hold children.
- **Determinism** — `serialize_html` of the same tree + root yields byte-identical output every call; attribute order is
  insertion order; `&` `<` `>` are escaped in text, `&` `<` `>` `"` in attribute values.

## Strategic Approach

### Solution Direction

A single-file-per-concept `domain/` module tree with one `application/` service (`serialize`). Data flows one way:
`DomTree` methods build and mutate an in-memory arena under invariant checks → `Descendants` / `Ancestors` read it →
`serialize_html` folds a subtree to `String`. Zero I/O, zero deps, `#![forbid(unsafe_code)]`. The crate compiles and
tests with `--no-default-features` (there are no features and no deps — the "Domínio sem engine" gate is green by
construction, and CI locks it with a `cargo tree -p dom` emptiness assertion).

### Key Design Decisions

- **Arena `Vec<Slot>` vs `Rc`/`RefCell` node graph**: arena. Trade-off: `NodeId` indirection on every access, and a
  removed id can dangle → `NodeNotFound`. Gain: no reference cycles to leak, `DomTree` is one owned value the Skeleton
  holds (`ADR-0003`), cheap to pass, and I1 can wrap the whole tree in one `Rc<RefCell<_>>` rather than per-node cells.
  → **arena**, matching decision 2.2.
- **Tombstone without index reuse vs generational `NodeId`**: tombstone only. Trade-off: a long-lived tree that churns
  nodes grows its `Vec` monotonically. Gain: `NodeId` stays a bare `u32`, exactly as `ADR-0010`/`CLAUDE.md` write it; no
  ABA class of bug. → **tombstone**, generational index deferred to v0.9 / C-13 (decision 2.2, risk table).
- **Explicit-stack traversal vs recursion**: explicit `Vec<NodeId>` stack. Trade-off: iterator state is a little more
  code. Gain: satisfies Object Calisthenics rule 1 (one indentation level) and cannot stack-overflow on a hostile-depth
  tree — relevant once `core/html` feeds the DOM from the network (v0.3+). → **explicit stack** (decision 2.3).
- **`remove` semantics**: `remove(node)` detaches `node` then tombstones `node` **and its whole subtree** (iterative
  post-order over the explicit stack). Alternative — tombstone only `node` and orphan the children — leaves invariant 5
  violated (children with a `parent` pointing at a `Tombstone`). → **cascade tombstone**.
- **`DomError` variant set**: the report §2.4 lists six illustrative variants. A total, invariant-complete error model
  needs four more, all documented as refinements: `InvalidAttributeName(String)` (attribute-name validation has to
  reject something), `NotCharacterData(NodeId)` (text op on a non-text node), `CannotHaveChildren(NodeId)` (parent is
  `Text`/`Comment`), and keeping the enum `#[non_exhaustive]` so I1's `DomError → EngineError::Dom` mapping never
  breaks. Final set: `NodeNotFound(NodeId)` · `WouldCycle` · `SelfParent` · `CannotDetachDocument` ·
  `CannotHaveChildren(NodeId)` · `InvalidTagName(String)` · `InvalidAttributeName(String)` · `NotAnElement(NodeId)` ·
  `NotCharacterData(NodeId)`.
- **`serialize_html` void elements**: handle the standard HTML void set
  (`area base br col embed hr img input link meta param source track wbr`) — emit `<br>` with no close tag. Alternative
  — always open+close — produces `<br></br>`, which is wrong HTML. → **honour the void set** (small, fixed,
  deterministic).
- **`serialize_html` signature**: returns `Result<String, DomError>` (a stale `root` id → `NodeNotFound`), not a bare
  `String`. The report writes `-> String`; a fallible signature is the honest one and costs the caller one `?`.

### Alternatives Considered

- **Depend on `engine` for a shared `DomError`/value type** — rejected: collapses decision 2.1 and drags `rhai` into
  `html`/`css`/`js` transitively (`overview.md:85` / `CLAUDE.md` are corrected to `None` in the I1 delivery, not here).
- **`indextree` / `ego-tree` crate** — rejected: adds a dependency to a crate whose entire point is zero deps, and hides
  the invariants F3 exists to own in the Skeleton.
- **Generational `NodeId(u32, u32)` now** — rejected for v0.2: not needed for the micro-deliverable, and it changes the
  newtype `ADR-0010` fixes literally. Deferred to C-13.

## Risk & Gap Analysis

### Requirement Ambiguities

- **`node.text` getter/setter on which kinds** — the report's §2.5 table shows `node.text` for both read and write. F3
  scopes `text` to `NodeKind::Text` only (`NotCharacterData` otherwise); `Comment` content is read/written through
  separate accessors. I1's binding surface follows F3.
- **Does `serialize_html` emit a doctype for `Document`** — no. v0.2 serialises the tree as-is; `<!DOCTYPE html>` is a
  parser concern (F5), not a tree concern. Documented as a limitation.
- **Whitespace / pretty-printing** — none. Output is the minimal concatenation, deterministic. No indentation.

### Edge Cases

- **`append_child` of a node onto its own current parent** — must be idempotent-ish: detach (removes from `Children`)
  then re-append at the end. Net effect: the child moves to last position. Tested.
- **`insert_before` with an `anchor` that is not a child of `parent`** — `NodeNotFound` (the anchor is not locatable in
  `parent.children`), tree unchanged.
- **`remove` of a node whose subtree contains the node passed to a later call** — the later call sees `NodeNotFound`
  (already tombstoned). Tested.
- **Traversal starting from a `Tombstone` / out-of-range id** — the iterator yields nothing; it does not panic.
- **`serialize_html` of a `Comment` containing `-->`** — v0.2 emits the content raw (no comment-close escaping). Noted
  as a known limitation; `core/html` will never produce such a comment, and the muscle script author is
  trusted-but-fallible (`PRD-002 §2.2`).
- **`TagName` with uppercase / trailing digits** — lowercased on construction; digits after the first char are allowed
  (`h1`), a leading digit is rejected.

### Technical Risks

- **Object Calisthenics rule 1 + rule 2 across the arena mutators** — acyclicity, single-parent and `Children`/`parent`
  coherence with no `else` and one indentation level produce many small private helpers (`slot`, `slot_mut`,
  `node_data`, `node_data_mut`, `detach_from_parent`, `is_ancestor`). Mitigation: budget for the helper count; the
  report risk 2 already flags F3 can touch 18 d because of this.
- **Two sequential `&mut` borrows of the same `Vec<Slot>`** — moving a child between parents needs to write the old
  parent's `Children`, the new parent's `Children`, and the child's `parent`. Do it as three ordered single-index
  operations (`split_at_mut` only if two indices are needed at once; otherwise sequential `slot_mut(i)`), never two
  overlapping `&mut`, never `unsafe`.
- **`is_ancestor(candidate, node)` cost** — O(depth) per `append_child`. Acceptable for v0.2 (trees are script-built and
  shallow); `core/html`-fed deep trees are v0.3+ and still O(depth), which is fine.
- **Serializer stack growth** — the explicit work-stack holds at most O(open ancestors + pending siblings); bounded by
  tree size, same as recursion would be but without the call-stack limit.

### Acceptance Criteria Coverage

| AC (source)                                                                                      | Addressable? | Gaps / Notes                                                                                                |
| ------------------------------------------------------------------------------------------------ | ------------ | ----------------------------------------------------------------------------------------------------------- |
| `append_child` creating a cycle → `DomError::WouldCycle`, tree unchanged (report §5)             | Yes          | `is_ancestor` check before any mutation; test asserts `slots` length + links unchanged                      |
| Attaching a node that already has a parent removes it from the old `Children` (§5)               | Yes          | Single-parent move in `append_child` / `insert_before`                                                      |
| `detach` / `remove` of `Document` → `DomError::CannotDetachDocument` (§5)                        | Yes          | id compared to `self.document` first                                                                        |
| `Descendants` pre-order deterministic; `Ancestors` ends at the root (§5)                         | Yes          | explicit-stack iterators; tests assert exact `Vec<NodeId>`                                                  |
| `build → serialize_html → String` matches expected; attrs in insertion order; `&<>` escaped (§5) | Yes          | `serialize_html` + `AttributeMap` insertion order + escape table                                            |
| `cargo test -p dom --no-default-features` compiles and passes — links no engine (§5)             | Yes          | zero deps; CI adds `cargo tree -p dom` emptiness check (wired in the I1 delivery alongside the other gates) |
| C-03 (`DomNode` scriptable) — `PRD-002:89`                                                       | **No (I1)**  | F3 delivers the pure tree only; the `NodeHandle` + `document` binding is I1                                 |

### Out of Scope for F3

Per report §2.10: no `Origin` / `WEB_CONTENT` / per-tab isolation (F7); no `core/html` / `core/css` / `core/graphics`
work; no hot-reload; no `criterion`; no generational `NodeId`. Also: no `EngineError`, no `rhai`, no `engine` dependency
(decision 2.1); the `DomError → EngineError::Dom` map and the `core/dom → None` doc corrections are **I1** deliverables.
