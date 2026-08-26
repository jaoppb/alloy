# SPDD Analysis: DOM Tree Arena Hierarchy & Mutations (core/dom)

## Original Business Requirement

### ROADMAP-IMPLEMENTACAO-V1: Fase F3 (Trilha B) & Critério C-03 (PRD-002:89)

Implement the DOM core subsystem in `core/dom` based on the arena pattern:

- Define `NodeId`, `DomNode`, `DomTree` arena, `Children` first-class collection, and `TagName` newtype.
- Enforce strict tree invariants: acyclicity (a node cannot be its own ancestor), single-parent linkage, and valid node
  references.
- Support core DOM mutations: node creation (Element, Text, Document, Comment), `append_child`, `insert_before`,
  `remove_child`.
- Provide traversal iterators (ancestors, depth-first pre-order traversal).
- Provide application/infrastructure ports to expose DOM inspection and mutation across the script runtime boundary
  (**C-03**).

Acceptance Criteria & Invariants:

- **C-03**: Registered Rust domain struct (`DomNode`) readable and mutable from script engine.
- **Tree Invariants**: Acyclicity test passes; detaching and re-attaching preserves parent/child integrity.
- **Object Calisthenics (ADR-0010)**: Zero naked primitives (`NodeId`, `TagName`), first-class collections (`Children`,
  `AttributeMap`), no `else`, `#![forbid(unsafe_code)]`.

---

## Domain Concept Identification

### Existing Concepts (from codebase)

- `engine` (`core/engine`): Abstract scripting ports (`RuntimeEngine`, `ExecutionContext`, `EngineValue`, `EngineError`,
  `Identifier`).
- `rhai-runtime` (`core/runtime/rhai`): Concrete Rhai backend ready to consume domain structures for C-03.

### New Concepts Required

- `NodeId`: Unique integer identifier newtype referencing a node inside the arena.
- `TagName`: Validated, normalized (lowercase) element tag name (e.g. `div`, `span`, `body`).
- `AttributeName` & `AttributeValue`: Strongly typed attribute key/value pair.
- `AttributeMap`: First-class collection wrapping attributes of an element.
- `Children`: First-class collection wrapping child `NodeId` list and maintaining ordering.
- `NodeData`: Enum distinguishing `Document`, `Element`, `Text`, and `Comment`.
- `DomNode`: Entity combining identity (`NodeId`), structural hierarchy (`Option<NodeId>` parent, `Children`), and
  payload data (`NodeData`).
- `DomTree`: Aggregate root arena managing node allocations, parent-child invariants, cycle prevention, and traversals.
- `DomError`: Typed domain error enum (`NodeNotFound`, `CycleDetected`, `InvalidHierarchy`, `InvalidTagName`).

### Key Business Rules

- **Cycle Prevention**: Appending or inserting a node `A` under node `B` where `B` is a descendant of `A` (or `A == B`)
  is illegal and must return `DomError::CycleDetected`.
- **Single Parent**: A node can only have at most one parent. Moving an existing node to a new parent automatically
  detaches it from its previous parent first.
- **Safe Arena Indexing**: Operations verify that the target `NodeId` exists and has not been freed/removed.
- **Zero Unsafe**: Pure Rust memory management using an indexed vector arena (`Vec<Option<DomNode>>`), eliminating
  dangling pointers without raw pointers.

---

## Strategic Approach

### Solution Direction

- Implement Clean Architecture in `core/dom`:
    - `src/domain/`: `node_id.rs`, `tag_name.rs`, `attribute.rs`, `children.rs`, `node_data.rs`, `node.rs`, `tree.rs`,
      `error.rs`.
    - `src/application/`: `service.rs` (tree serialization and query helpers).
    - `src/infrastructure/`: `bridge.rs` (native function bindings to expose DOM reads and mutations to
      `engine::ExecutionContext`).
    - `src/lib.rs`: Public facade re-exporting the ubiquitous DOM language.
- Add `engine = { workspace = true }` to `core/dom/Cargo.toml`.

### Key Design Decisions

- **Arena vs `Rc<RefCell<Node>>`**: Use indexed vector arena (`Vec<Option<DomNode>>`) with `NodeId(u32)` or
  `NodeId(usize)`. Avoids reference counting cycles, memory leaks, and runtime borrow panics.
- **Pre-order DFS Iterator**: Expose depth-first iterator for layout calculation (`core/css`, `core/graphics`) and tree
  printing.
- **Script Bridge**: Provide `register_dom_bindings(ctx: &mut dyn ExecutionContext, tree: Arc<Mutex<DomTree>>)` closing
  C-03.

### Alternatives Considered

- _Raw pointer / unsafe doubly-linked tree_: Rejeitado categoricamente pelo requisito de segurança N-02 e
  `#![forbid(unsafe_code)]`.
- _Shared `Rc<RefCell<Node>>` graph_: Rejeitado por overhead de alocação no heap por nó e risco de vazamento de memória
  por ciclos de referências cíclicas.

---

## Risk & Gap Analysis

### Requirement Ambiguities

- Index representation: Using `u32` vs `usize` for `NodeId`. `u32` is memory-efficient and maps cleanly to `NodeId(u32)`
  from ADR-0010, while easily casting to `usize` for vector indexing.

### Edge Cases

- Removing the root node: `DomTree` must clear root reference if the root is removed.
- Inserting a node before itself: Must return `DomError::CycleDetected` or `InvalidHierarchy`.
- Detaching already orphaned node: Handled gracefully as a no-op.

### Technical Risks

- Tree depth leading to stack overflow during cycle checking: Mitigated by iterative ancestor traversal (`ancestors()`
  iterator) instead of deep recursive calls.

### Acceptance Criteria Coverage

| AC#         | Descrição                                                          | Endereçável nesta Fase (F3)? | Notas                                                          |
| :---------- | :----------------------------------------------------------------- | :--------------------------- | :------------------------------------------------------------- |
| **C-03**    | Struct de domínio (`DomNode`) legível e mutável a partir de script | Sim (com I1)                 | F3 cria o DOM e as pontes; teste de integração fecha com Rhai. |
| Invariantes | Aciclicidade, inserção, remoção e travessia da árvore DOM          | Sim                          | Coberto integralmente pela suíte de testes de `core/dom`.      |
