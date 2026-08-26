# SPDD Analysis: Runtime Hot-Reload Subsystem (core/engine & F11)

## Original Business Requirement

### ROADMAP-IMPLEMENTACAO-V1: Fase F11 (Trilha A) & PRD-004 (§Runtime Hot-Reload Subsystem)

Implement the runtime hot-reload subsystem in `core/engine` delivering **Release v0.4 ("Recarregamento atômico a
quente")**:

- Detect `.rhai` script modifications with debouncing (**C-10**).
- Compile successful script edits in background and swap atomically (**C-11**).
- When a script has syntax errors, preserve the active AST and log diagnostics (**C-12**).
- Ensure active DOM and application state remain 100% intact after multiple hot-reloads (**C-13**).

---

## Domain Concept Identification

### Existing Concepts (from codebase)

- `RuntimeEngine`, `ExecutionContext` (`core/engine`).
- `DomTree`, `DomNode`, `NodeId` (`core/dom`).
- `RhaiEngine`, `RhaiContext` (`core/runtime/rhai`).

### New Concepts Required

- `DebounceDuration`: Value object encapsulating watcher debounce timing (default 50ms per PRD-004:42).
- `HotReloadStatus`: Outcome of a hot-reload attempt:
    - `Success { version: u64 }`
    - `CompilationError { error: String, previous_version: u64 }`
    - `Unchanged`
- `AtomicScriptSlot<AST>`: Thread-safe atomic slot holding the active compiled AST.
- `HotReloadCoordinator<E: RuntimeEngine>`: Orchestrator managing background compilation, atomic swap, and error
  rollback.
- `ScriptWatcher`: File monitor with debouncing for `.rhai` files.

### Key Business Rules

- **Skeleton & Muscle Ground Truth**: Rust memory holds all domain state (DOM, layout, network). Scripts are stateless
  policy; script scope reset never corrupts the DOM tree (**C-13**).
- **Atomic Pointer Swap**: Script replacement is instantaneous and atomic (`Arc<AST>`). Subsystems never execute
  half-compiled scripts (**C-11**).
- **Rollback on Syntax Error**: If an edit contains syntax or compilation errors, the previous AST remains active and
  the error diagnostic is reported without interrupting browser execution (**C-12**).
- **Debounce Invariant**: File modifications within the debounce window (50ms) are coalesced into a single reload
  (**C-10**).

---

## Strategic Approach

### Solution Direction

- In `Cargo.toml`: Add `notify = "8.0"` to `[workspace.dependencies]`.
- In `core/engine/Cargo.toml`: Add `notify = { workspace = true }`.
- In `core/engine/src/domain/`:
    - `hot_reload.rs`: `DebounceDuration`, `HotReloadStatus`, `ReloadableScript`.
- In `core/engine/src/application/`:
    - `hot_reload.rs`: `AtomicScriptSlot`, `HotReloadCoordinator`, `ScriptWatcher`.
- In `core/engine/tests/`:
    - `hot_reload_conformance.rs`: Comprehensive test suite verifying C-10, C-11, C-12, C-13.

---

## Acceptance Criteria Coverage

| AC#      | Descrição                                                   | Endereçável nesta Fase (F11)? | Notas                                            |
| :------- | :---------------------------------------------------------- | :---------------------------- | :----------------------------------------------- |
| **C-10** | Watcher detecta modificação de `.rhai` com debounce         | Sim                           | `ScriptWatcher` com debounce de 50ms.            |
| **C-11** | Edição válida compila em background e troca atomicamente    | Sim                           | `HotReloadCoordinator` e `AtomicScriptSlot`.     |
| **C-12** | Script com erro de sintaxe não substitui o AST ativo e loga | Sim                           | Rollback automático preservando versão anterior. |
| **C-13** | DOM e estado intactos após múltiplos hot-reloads            | Sim                           | Teste com mutações no DOM e múltiplos swaps.     |
