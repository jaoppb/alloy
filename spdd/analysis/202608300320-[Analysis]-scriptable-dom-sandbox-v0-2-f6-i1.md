# SPDD Analysis — v0.2 F6 + I1: the capability sandbox and the scriptable DOM

> Phase 0 artefact for the second half of v0.2 "DOM scriptável e contido". Builds on the delivered v0.1 and on F3
> (`spdd/prompt/202608300315-[Feat]-dom-dom-tree-arena-v0-2-f3.md`). Consolidated from
> `docs/reports/IMPLEMENTACAO-DETALHADA-V0-2.md` §2.5–2.9 and §3 (F6 + I1 steps), `PRD-003` (§4 fault isolation, §5
> acceptance), `PRD-002` §2.1 / §4.1, `ADR-0004` (capability sandboxing + fallback), `ADR-0011` (Replaceable Port
> Contract, item 2 — the object-safe companion), and `docs/architecture/runtime-engine-port-contract.md`.

## Original Business Requirement

From `ROADMAP-IMPLEMENTACAO-V1.md` §3.2 (F6), §3.3 (I1) and `docs/reports/IMPLEMENTACAO-DETALHADA-V0-2.md`:

> **F6 — Sandbox**: `register_guarded_binding`, tabela `Binding`, contextos isolados, fallback, panic hook; ADR-0013 +
> companion. Fecha **C-06**, **C-07**, **C-08**, **C-09**. Esforço `[modelado]`: 12–18 d.
>
> **I1 — `dom_bindings.rs`**: `NodeHandle` (`CustomType`), global `document`, mapeamento `DomError → EngineError::Dom`.
> Fecha **C-03** — script monta e muta o DOM, host lê de volta. Esforço `[modelado]`: 3–6 d.
>
> Decisão 2.5 — superfície scriptável: o script manipula um `NodeHandle { tree: Rc<RefCell<DomTree>>, id: NodeId }`,
> `Clone` barato, `impl CustomType` via `build_type`. Superfície da v0.2: `document` (global, `DOM_READ`); `node.tag` /
> `node.text` (getter, `DOM_READ`); `node.children()` (`DOM_READ`); `node.create_element(tag)` (`DOM_MUTATE`);
> `node.append_child(child)` (`DOM_MUTATE`, invariantes 1–3); `node.text = "…"` (setter, `DOM_MUTATE`);
> `node.set_attribute(name, value)` (`DOM_MUTATE`). A mutação passa por `tree.try_borrow_mut()` — falha de empréstimo
> vira `EngineError::Dom { reason: "DOM ocupado" }` em vez de `panic` de `RefCell`. `Rc<RefCell<_>>` é o idioma correto:
> `rhai` é single-threaded por `eval`, e `ExecutionContext` (`PRD-002:45`) **não** exige `Send + Sync`. O `RhaiContext`
> passa a `!Send`.
>
> Decisão 2.6 — a guarda de capability, ponto único, resolvida na criação do contexto: C-06 exige verificação em
> **todo** binding. `RhaiContext::register_guarded_binding(name, required: Capability, handler)` embrulha `handler` numa
> closure que faz `self.capabilities.contains(required)?` antes de chamar. `register_fn` cru fica reservado a funções
> **puras**. Uma tabela `&[Binding { name, required, handler }]` é percorrida uma vez ao montar o contexto, e o teste de
> conformidade itera essa mesma tabela afirmando que nenhum binding de DOM escapa da guarda. O `CapabilitySet` é
> capturado **por valor** (`bitflags` é `Copy`) em `create_context` — sem relookup por chamada; a verificação é `and` de
> bits + branch (orçamento `<10μs` por hook, `PRD-001:96`). Revogação em runtime não existe na v0.2 (`ADR-0004` fixa as
> capabilities na criação do contexto).
>
> Decisão 2.7 — ciclo de vida e concorrência: o `DomTree` é do host Rust (Skeleton); o `RhaiContext` guarda um clone do
> `Rc` e o host lê a árvore mutada após `eval` retornar. Um `RhaiContext` por subsistema, preso a uma thread (`!Send`).
> Nenhum binding da v0.2 chama de volta para o script; `try_borrow_mut` é a rede de segurança. Script abortado por
> limite deixa o `DomTree` parcial — o fallback parte de uma árvore **limpa**.
>
> Decisão 2.8 — C-09, o fallback, não só o trapping: o trapping (`catch_unwind` → `ScriptPanic`) já existe de F2. A v0.2
> adiciona o **fallback** de `PRD-003:66-69`: (1) `eval` devolve `Err(...)`; (2) o host registra o diagnóstico — caminho
> do script, linha/coluna quando houver, variante — em `stderr` (o barramento de eventos do DevTools fica para depois);
> (3) o host executa `scripts/default_dom.rhai` embarcado, num contexto guardado novo, sobre um `DomTree` limpo; (4) se
> o próprio fallback falhar, uma rotina Rust mínima monta `<html><body></body></html>` e o processo segue; (5) `alloy`
> encerra com código 0. `panic = "abort"` continua proibido. Um panic hook local, instalado só durante `eval` e removido
> logo após, captura a localização e evita poluir `stderr` com o backtrace padrão.
>
> Decisão 2.9 — ADR-0013, companion object-safe de `RuntimeEngine`: `eval<T>` e os `register_*<…>` genéricos tornam
> `RuntimeEngine` não object-safe → não existe `Box<dyn RuntimeEngine>`. `ADR-0011:67-69` já exige "uma forma companion
> object-safe" para todo port. ADR-0013 adiciona a `core/engine/src/application/` um par object-safe — todos os métodos
> `-> Result<_, EngineError>`: `DynRuntimeEngine` (`create_context_dyn`, `eval_value`, …), `DynExecutionContext`
> (`set_value`, `call`, `reset_scope`, …). Sem método genérico na fronteira `dyn`; `eval::<T>` vira **função livre** em
> `dyn_bridge.rs`. Blanket impl `impl<E: RuntimeEngine> DynRuntimeEngine for E` — `MockEngine` e `RhaiEngine` ganham o
> companion de graça. Numeração: **0013** (0012 está reservado para o motor JS, `ADR-0011:108`).
>
> Decisão 2.4 (parte I1) — `dom_bindings.rs` mapeia `DomError` para uma variante nova
> `EngineError::Dom { operation: String, reason: String }` — adição mínima a `core/engine/src/domain/error.rs`,
> justificada porque `Binding(String)` (v0.1) perde a distinção entre "capability negada" e "operação de DOM inválida".
>
> Divergência de documentação a corrigir nesta entrega: `overview.md:85` e `CLAUDE.md` declaram `core/dom` com
> dependência `engine`. A decisão 2.1 mantém `core/dom` **sem dependência alguma** — passam a ser corrigidas para
> `None`.

## Domain Concept Identification

### Existing Concepts (from codebase)

- **`RuntimeEngine` / `ExecutionContext`** (`core/engine/application/ports.rs`) — the v0.1 port. The required
  `ExecutionContext` methods (`capabilities`, `register_type_erased`, `register_native_fn`, `set_value`, `get_value`,
  `call_function_value`, `reset_scope`) are **already object-safe** (`dyn ExecutionContext` compiles today — contract
  record §2). Only `RuntimeEngine` is not (associated types by value + generic sugar).
- **`CapabilitySet`** (`core/engine/domain/capability.rs`) — `Copy` newtype over `Capability` bitflags; already carries
  `require(needed) -> Result<(), EngineError::PermissionDenied>` and `contains`. This is the single check every guarded
  binding calls (its doc comment already says so — "the single check every guarded binding will call in F6").
- **`EngineError`** — `#[non_exhaustive]`; variants `Compilation`, `ExecutionLimitExceeded`, `PermissionDenied`,
  `TypeMismatch`, `Conversion`, `Binding`, `ScriptRuntime`, `ScriptPanic`. `PORT_SCHEMA_VERSION = 1`.
- **`NativeFn`** = `Arc<dyn Fn(&[EngineValue]) -> Result<EngineValue, EngineError> + Send + Sync>` — "the shape F6 will
  wrap with a capability guard" (its own doc comment).
- **`RhaiEngine` / `RhaiContext` / `RhaiCompiledScript`** (`core/runtime/rhai/infrastructure/`) — v0.1. `RhaiContext`
  already owns its own `rhai::Engine` + `rhai::Scope<'static>` + `CapabilitySet` + a `HashMap<String, NativeFn>` +
  `registered_type_names`. `evaluate_ast` already wraps `eval_ast_with_scope` in `catch_unwind(AssertUnwindSafe(…))` →
  `EngineError::ScriptPanic`.
- **`RhaiContext::register_custom_type::<T: EngineType + rhai::CustomType>()`** — v0.1 adapter extension using
  `engine.build_type::<T>()`. The fixture `FixtureNode` (`tests/fixture_node.rs`) proves the mechanism of C-02.
- **`engine::conformance::run_core_suite`** — the backend-agnostic suite (`ADR-0011` item 6); `RhaiEngine` and
  `MockEngine` both run it.
- **`DomTree` + `serialize_html` + `DomError`** (`core/dom`, F3) — the pure tree this phase makes scriptable.
- **`alloy` binary** — v0.1 CLI; `--script <path>` currently runs under `CapabilitySet::empty()` and prints the value.

### New Concepts Required

- **`GuardedBinding`** — `{ name: &'static str, arity: Arity, required: Capability, handler: NativeFn }`. A table of
  these is walked once per context build; the C-06 conformance sweep walks the same table.
- **`RhaiContext::register_guarded_binding`** — the single chokepoint: takes `&FunctionName`, arity, required
  `Capability`, and wraps a `handler` so `capabilities.require(cap)?` runs before it. Records `(FunctionName, required)`
  for the sweep. `register_native_fn` (unguarded) stays for **pure** functions only.
- **`NodeHandle`** — `{ tree: Arc<Mutex<DomTree>>, id: NodeId, capabilities: CapabilitySet }`. The script-visible DOM
  node. `Clone` is cheap (`Arc` bump + `Copy` fields). `impl engine::EngineType` (script name `"Node"`) +
  `impl rhai::CustomType`. Every method checks its own capability (`DOM_READ` for reads, `DOM_MUTATE` for mutations) —
  the capability is baked in at handle-creation time because `ADR-0004` fixes it for the context's life.
- **`NODE_HANDLE_BINDINGS`** — `&[(&str, Capability)]` manifest of every `NodeHandle` method and its required
  capability. The C-06 sweep drives this: for each entry, a handle built with `CapabilitySet::empty()` must return
  `PermissionDenied`.
- **`RhaiContext::bind_dom(Rc<RefCell<DomTree>>)`** — concrete method, **outside** the port trait (report §3 I1):
  registers `NodeHandle` as a custom type and sets the global `document` handle stamped with `self.capabilities()`.
- **`EngineError::Dom { operation: String, reason: String }`** — new port error variant. Bumps `PORT_SCHEMA_VERSION` →
  **2**; migration note in `PRD-002`; contract record updated.
- **`DynRuntimeEngine` / `DynExecutionContext` / `DynCompiledScript`** (`core/engine/application/dyn_bridge.rs`) — the
  ADR-0013 object-safe companion. `DynExecutionContext` is exactly the object-safe core of `ExecutionContext`;
  `DynRuntimeEngine` boxes the context and downcasts it back inside the blanket impl. `eval_typed::<T>` is a free
  function. Purely additive — changes no existing signature.
- **`fallback` module** (`core/runtime/rhai/infrastructure/fallback.rs`) — `run_with_fallback(...)`: primary script → on
  `Err`, stderr diagnostic → embedded `default_dom.rhai` on a **clean** tree → on `Err`, a minimal Rust
  `<html><body></body></html>`. Scoped panic hook around each eval.
- **`scripts/default_dom.rhai`, `scripts/hello_dom.rhai`** — the embedded fallback and the micro-deliverable example.
- **ADR-0013 MADR** + `docs/adr/README.md` row.

### Key Business Rules

- **Every DOM binding is capability-guarded** (C-06). No path from a script to `DomTree` exists that did not first pass
  `CapabilitySet::require`. Reads need `DOM_READ`; mutations need `DOM_MUTATE`.
- **A denied capability is `EngineError::PermissionDenied`** (C-07), carrying the missing flag — never a silent no-op,
  never a different variant.
- **Contexts are isolated** (C-08): two contexts from one `RhaiEngine` share no scope variable, no registered binding,
  no capability set, and — new in v0.2 — no `DomTree`. A fault in one does not disturb the next `eval` of the other.
- **A panicking script never aborts the host and the fallback takes over** (C-09): `catch_unwind` → `ScriptPanic` →
  stderr diagnostic → `default_dom.rhai` on a clean tree → Rust minimal document → process continues, `alloy` exits 0.
- **The engine port carries exactly one error type** (`ADR-0011` item 4): `DomError` is mapped to `EngineError::Dom` in
  the adapter; `core/dom` never names `EngineError`.
- **Capabilities are fixed at context creation** (`ADR-0004`): no runtime grant or revoke; the handle can safely bake in
  the set copied from its context.
- **`DomTree` durable state belongs to the Skeleton** (`ADR-0003`, contract §5.1): the host holds an `Rc` clone and
  reads the mutated tree after `eval` returns; the context holds only script-local state.

## Strategic Approach

### Solution Direction

Two adapter modules and one `core/engine` module, plus small edits to existing files:

- `core/engine/src/application/dyn_bridge.rs` — the ADR-0013 companion (additive).
- `core/engine/src/domain/error.rs` — `+ EngineError::Dom`; `PORT_SCHEMA_VERSION 1 → 2`.
- `core/runtime/rhai/src/infrastructure/sandbox.rs` — `GuardedBinding`, `register_guarded_binding`, the context-side
  table, the guarded-binding-name registry for the sweep.
- `core/runtime/rhai/src/infrastructure/dom_bindings.rs` — `NodeHandle`, `NODE_HANDLE_BINDINGS`, `bind_dom`, the
  `DomError → EngineError::Dom` map.
- `core/runtime/rhai/src/infrastructure/fallback.rs` — `run_with_fallback`, the scoped panic hook, the Rust minimal
  document.
- `core/runtime/rhai/src/infrastructure/context.rs` / `engine.rs` — `RhaiContext` gains the `!Send`
  `Rc<RefCell<DomTree>>` hook and the guarded table; `RhaiEngine` stays `Send + Sync` (it wraps only a stateless
  `rhai::Engine` + `ExecutionLimits`).
- `alloy/src/main.rs` — `--script` now runs the DOM demo through `run_with_fallback` with `profiles::dom_parser()` and
  prints `serialize_html`; on any error it falls back and still exits 0.
- Docs: ADR-0013 + README row; `PRD-002` amendment (companion + `EngineError::Dom`); contract record (item 2 → done,
  schema 2); `overview.md` + `CLAUDE.md` (`core/dom` deps → `None`, v0.2 criteria closed); a sync note appended to
  `IMPLEMENTACAO-DETALHADA-V0-2.md`.
- CI: assert `cargo tree -p dom` is dependency-free; add the panic-injection matrix test as a **blocking** job (roadmap
  §5 — "passa a bloquear na v0.2"); the `dyn` conformance runs inside `cargo test --workspace`.

Data flow: `alloy` builds one `Rc<RefCell<DomTree>>` → `RhaiEngine::create_context(profile)` →
`ctx.bind_dom(rc.clone())` registers `NodeHandle` + the `document` global → `eval_value(script)` mutates the tree
through guarded handle methods → on return `alloy` reads `rc.borrow()` and prints `serialize_html`. On any `Err`,
`run_with_fallback` swaps in a clean tree and the embedded default.

### Key Design Decisions

- **Where the capability check lives for `NodeHandle` methods** — `rhai::TypeBuilder` registers methods directly on the
  engine via `build_type`, _not_ through `register_guarded_binding`. Two options: (a) `NodeHandle` carries a
  `CapabilitySet` and each method calls `require` itself, with a `NODE_HANDLE_BINDINGS` manifest the sweep verifies; (b)
  expose every DOM op as a top-level guarded free function (`dom_tag(node)`, `dom_append_child(p, c)`) and use
  `build_type` only to pass the opaque handle. → **(a)**. It keeps `node.tag()` / `node.append_child(c)` ergonomics,
  keeps the check inside the DOM adapter surface, and the manifest gives the sweep the same "walk the table" guarantee
  `register_guarded_binding` gives for non-handle bindings. This is the solid path; (b) trades script ergonomics for a
  single registrar and is rejected.
- **`NodeHandle` reads are methods, not property getters** — `rhai`'s `TypeBuilder::with_get` closure returns
  `V: Variant`, so it **cannot** return `Result` and cannot surface a `DomError` or a `PermissionDenied`. A getter that
  silently returns `""` on a type error or a missing capability violates the solid-path error model. → reads are
  `node.tag()`, `node.text()`, `node.children()`, `node.get_attribute(name)`, each returning
  `Result<_, Box<EvalAltResult>>`; mutations likewise are methods (`node.set_text(s)`, not `node.text = s`). The report
  §2.5 table's getter/setter ergonomics are dropped for correctness — a documented deviation.
- **`DynExecutionContext` = the object-safe core of `ExecutionContext`; `DynRuntimeEngine` downcasts** — the blanket
  `impl<T: ExecutionContext> DynExecutionContext for T` is a trivial delegation and needs no `Any`. `DynRuntimeEngine`'s
  `eval_value` receives `&mut dyn DynExecutionContext`; to reach `E::Context` it downcasts via an `as_any_mut` on
  `DynExecutionContext` (blanket-provided). The blanket `impl<E> DynRuntimeEngine for E` is bounded
  `where E: RuntimeEngine, E::Context: 'static` — which `RhaiContext` and `MockEngine`'s context both satisfy — so **no
  change to `RuntimeEngine` or `ExecutionContext` is required**. The companion is genuinely additive; the only port
  surface change in v0.2 is `EngineError::Dom`.
- **`RhaiContext` becomes `!Send`; `RhaiEngine` stays `Send + Sync`** — `PRD-002:35` requires `Send + Sync` only on
  `RuntimeEngine`; `PRD-002:45` / contract §5.2 explicitly do **not** require it on `ExecutionContext`. Holding
  `Rc<RefCell<DomTree>>` on the context is therefore legal and is the right idiom (`rhai` eval is single-threaded). The
  conformance suite and `MockEngine` must not assume `RhaiContext: Send`.
- **Scoped panic hook location** — install/restore `std::panic::set_hook` around each `eval` **inside
  `run_with_fallback`** (and the panic-injection test helper), via a `Drop` guard that restores the previous hook.
  Alternative — hoist it into `RhaiEngine::evaluate_ast` so every eval is quiet — is cleaner long-term but widens a v0.1
  function; deferred. The hook only suppresses the default backtrace on stderr and records the panic location; the
  actual trap stays `catch_unwind` in `evaluate_ast`.
- **Fallback starts from a clean `DomTree`** (contract §5.4 / report §2.7) — a limit-aborted or panicking primary script
  may leave the tree half-built; `run_with_fallback` constructs a **new** `DomTree` for the `default_dom.rhai` attempt,
  never reusing the partial one.
- **`alloy --script` capability profile** — `profiles::dom_parser()` (`DOM_READ | DOM_MUTATE`). The demo needs to build
  and read a tree; it needs nothing else. A future `--caps` flag is out of scope.
- **`EngineError::Dom` vs reusing `Binding`** — `Binding(String)` already means "bad name / arity / unknown function";
  overloading it for "the script tried an illegal DOM mutation" erases the distinction a fallback handler and DevTools
  need. → new variant (report §2.4).

### Alternatives Considered

- **Per-node `Rc<RefCell<Node>>` instead of one `Rc<RefCell<DomTree>>`** — rejected: multiplies borrow-conflict surface,
  and the arena (F3) is already one owned aggregate; one cell around the whole tree matches it.
- **Runtime capability revocation** — rejected for v0.2: `ADR-0004` fixes capabilities at context creation; revocation
  is not a stated requirement and would break the "capture by value, no relookup" budget argument.
- **Making `RuntimeEngine` object-safe directly** (drop the generic sugar) — rejected: it is the frozen `F1` surface
  (`ADR-0011` item 7); the companion is the sanctioned additive path (`ADR-0011:67-69`).
- **DevTools event-bus wiring for the fault log** — deferred: `devtools` is a stub in v0.2 (`PRD-003:67`, contract
  §5.6); the diagnostic goes to `stderr` for now.

## Risk & Gap Analysis

### Requirement Ambiguities

- **"Verificação em todo binding" scope** — does C-06 cover pure helper functions (e.g. a `log(msg)` with no side
  effect)? Reading: no — the sweep asserts every **DOM** binding is guarded; a genuinely pure fn may use `register_fn`.
  The manifest is the source of truth for what must be guarded.
- **`node.children()` return type** — a `rhai::Array` of `NodeHandle`. Each element carries the same `Rc` and the same
  baked-in `CapabilitySet`.
- **Does the fallback re-run on a _compile_ error of the primary script** — yes; `run_with_fallback` treats any
  `Err(EngineError)` from compile-or-eval identically (report §2.8 step 1 lists `Compilation` among them).
- **What does `alloy` print on the fallback path** — `serialize_html` of whichever tree `run_with_fallback` returns (the
  default or the Rust minimal), plus the diagnostic already written to stderr. Exit code 0.

### Edge Cases

- **`node.append_child(child)` where `child` came from a different `DomTree`** — `NodeHandle` compares `Rc::ptr_eq` on
  the two `tree` fields; mismatch →
  `EngineError::Dom { operation: "append_child", reason: "node belongs to another document" }`. (In v0.2 there is one
  tree per context so this is defence-in-depth, but the check is cheap and correct.)
- **Re-entrant borrow** — a getter and a setter both touching the cell in one script statement. No v0.2 binding calls
  back into the script, so this cannot deadlock; still, every method uses `try_borrow` / `try_borrow_mut` and maps
  failure to `EngineError::Dom { reason: "DOM busy" }` rather than letting `RefCell` panic.
- **Panic inside a `NodeHandle` method** (not just a top-level binding) — the panic-injection matrix covers **each**
  entry of `NODE_HANDLE_BINDINGS` plus each `GuardedBinding`: `panic!` in the handler → `eval` returns `ScriptPanic`,
  the test process stays alive, `run_with_fallback` runs the default.
- **`default_dom.rhai` itself panics or fails** — step 4: the Rust `minimal_document()` builds
  `<html><body></body></html>` directly on a fresh `DomTree`. This routine is not optional (report risk 4) and is
  unit-tested like any code.
- **A context with `DOM_READ` but not `DOM_MUTATE`** — getters work, `create_element` / `append_child` / `set_text` /
  `set_attribute` return `PermissionDenied`. This is the C-07 micro-deliverable.
- **`MockEngine` and the `dyn` path** — `MockEngine`'s context must satisfy `'static`; its conformance run now also
  drives `run_dyn_suite`.
- **`RhaiContext` no longer `Send`** — any test or helper that moved a context across a thread (none in v0.1, but the
  conformance suite must not add one) would stop compiling; verified by `cargo test --workspace`.

### Technical Risks

- **`NodeHandle` surface creep** (report risk 1) — test scripts will want `insert_before`, `remove_child`, cloning,
  selector queries. Each is another guarded method. v0.2 ships exactly the report §2.5 table (adjusted: reads as
  methods) plus `create_text` (needed to build text nodes) and `get_attribute`; anything else is a follow-up. The 3–6 d
  I1 estimate assumes the demo stays within this surface.
- **`rhai::CustomType` + `Result`-returning methods** — confirm `TypeBuilder::with_fn` accepts a closure returning
  `Result<T, Box<EvalAltResult>>` in the pinned `rhai = "=1.26.0"`. If a signature quirk appears, isolate it in
  `dom_bindings.rs` only (never leak `rhai` types outward).
- **`ADR-0013` touches a frozen surface** (report risk 3, `ADR-0011:83-85`) — the companion is additive (new module, new
  traits, blanket impls; no existing signature changes), but the formalities are mandatory: `PORT_SCHEMA_VERSION` bump
  (driven by `EngineError::Dom`, not the companion), a `PRD-002` migration note, and the contract-record update.
  Skipping them erodes the port contract's authority.
- **Scoped panic hook + parallel tests** — `set_hook` is process-global; two tests installing hooks concurrently race.
  Mitigation: the panic-injection matrix test runs its cases on a single thread (`#[test]` fn with an internal loop, or
  `--test-threads=1` for that target) and always restores via the `Drop` guard.
- **`<10μs` per guarded hook** — not measured until v0.5 (`criterion`, roadmap §5); the design keeps the guard an `and`
  of `Copy` bits + a branch with no allocation, so the budget is not _introduced_ as debt here.

### Acceptance Criteria Coverage

| AC (`PRD-003:76-79` / `PRD-002:89` / report §5)                                                                            | Addressable? | Gaps / Notes                                                                                                              |
| -------------------------------------------------------------------------------------------------------------------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------- |
| C-06 — capability verification at **every** native DOM binding                                                             | Yes          | `register_guarded_binding` chokepoint + `NODE_HANDLE_BINDINGS` manifest; conformance sweep walks both tables              |
| C-07 — unauthorized capability → `EngineError::PermissionDenied`                                                           | Yes          | `CapabilitySet::require` returns exactly that variant with the missing flag; test with a `DOM_READ`-only context          |
| C-08 — isolated `ExecutionContext` instances, separate scopes                                                              | Yes          | fresh `rhai::Engine` + `Scope` + `CapabilitySet` + `Rc<RefCell<DomTree>>` per context; three-part isolation test          |
| C-09 — panicking script does not abort the host and invokes the fallback handler                                           | Yes          | `catch_unwind` (v0.1) → `ScriptPanic`; `run_with_fallback` adds the stderr diagnostic + `default_dom.rhai` + Rust minimal |
| C-03 — `DomNode` readable and mutable from a Rhai script (`PRD-002:89`)                                                    | Yes (I1)     | `NodeHandle` + global `document`; host reads the mutated `DomTree` after `eval`                                           |
| `ADR-0011` item 2 — object-safe companion for the port                                                                     | Yes          | `DynRuntimeEngine` / `DynExecutionContext` / `DynCompiledScript` + `eval_typed`; contract record item 2 → done            |
| Roadmap §5 — panic-injection isolation gate becomes **blocking**                                                           | Yes          | new CI job runs the matrix test; any `panic = "abort"` profile fails it immediately                                       |
| `alloy --script build_dom.rhai` prints serialized HTML; `alloy --script panics.rhai` → stderr diagnostic, fallback, exit 0 | Yes (manual) | `run_with_fallback` wired into `main.rs`; also covered by an integration test on the library path                         |

### Out of Scope

Per report §2.10: no `Origin` / `WEB_CONTENT` profile / per-tab isolation (F7); no `core/html` / `core/css` /
`core/graphics` / `core/window` / `core/js`; no hot-reload watcher or `on_reload()`; no `criterion` benchmark; no
generational `NodeId`. Also: no DevTools event-bus wiring (stub); no runtime capability revocation; no `NodeHandle`
methods beyond the v0.2 surface (`insert_before`, `remove_child`, selectors, cloning are follow-ups).
