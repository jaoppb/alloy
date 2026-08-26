# SPDD Analysis: Hierarchical Capability Sandboxing & Fault Trapping (core/engine)

## Original Business Requirement

### PRD-003 & ADR-0004: Hierarchical Capability Sandboxing & Script Isolation

Implement strict hierarchical sandboxing and fault isolation within `core/engine`:

- Define subsystem capability profiles (`SubsystemProfile`) for HTML/DOM, CSS Cascade, Network Interceptor, and UI
  Window Manager.
- Ensure capability verification on every native function binding (**C-06**).
- Ensure unauthorized actions return `EngineError::PermissionDenied` (**C-07**).
- Ensure separate subsystems maintain isolated `ExecutionContext` instances with non-leaking scopes (**C-08**).
- Implement fault-trapped script execution catching panics and runtime errors with fallback handlers without crashing
  the host process (**C-09**).

---

## Domain Concept Identification

### Existing Concepts (from codebase)

- `Capability` & `CapabilitySet` (`core/engine`): Bitflags permissions for DOM, Network, FS, Graphics, Window, DevTools.
- `ExecutionContext` (`core/engine`): Port managing execution isolate state and variable scope.
- `RuntimeEngine` (`core/engine`): Port managing compilation and evaluation.
- `EngineError` (`core/engine`): Typed error enum including `PermissionDenied` and `RuntimeError`.

### New Concepts Required

- `SubsystemProfile`: Static domain factory producing standard `CapabilitySet` presets for subsystems:
    - `dom_parser()`: `DOM_READ | DOM_MUTATE`
    - `css_cascade()`: `DOM_READ | GRAPHICS_DRAW`
    - `network_interceptor()`: `NETWORK_FETCH | FS_WRITE_CACHE`
    - `ui_window()`: `WINDOW_MANAGE | GRAPHICS_DRAW | DOM_READ`
- `guarded_native_fn`: High-order combinator creating a `NativeFn` that verifies `ctx.capabilities().contains(required)`
  before delegating to the inner closure.
- `TrappedExecutor`: Safe execution orchestrator wrapping evaluations in `std::panic::catch_unwind`, mapping panics to
  `EngineError::PanicTrapped`, and invoking fallback handlers.
- `FallbackStrategy`: Enum or closure specifying how a subsystem recovers from a trapped script error.

### Key Business Rules

- **Non-Negotiable Permission Check (C-06, C-07)**: No native function registered via guarded bindings may execute if
  the calling context lacks the required capability flag.
- **Context Hermeticity (C-08)**: Scopes must be isolated per instance. Variables declared or modified in Context A must
  never be accessible or mutable from Context B.
- **Host Process Immunity (C-09)**: Script bugs or `panic!` inside closures must never unwind across the host boundary
  and terminate the browser process.

---

## Strategic Approach

### Solution Direction

- In `core/engine/src/domain/capability.rs`:
    - Add `SubsystemProfile` presets.
- In `core/engine/src/domain/error.rs`:
    - Add `PanicTrapped(String)` variant to `EngineError`.
- In `core/engine/src/application/sandbox.rs`:
    - Implement `guarded_native_fn(required: Capability, f: ...)` for binding safety.
    - Implement `TrappedExecutor::execute_with_fallback(...)` using `std::panic::catch_unwind`.
- In `core/engine/src/lib.rs`:
    - Re-export `SubsystemProfile`, `guarded_native_fn`, `TrappedExecutor`.
- Integration and Conformance Tests in `core/engine/tests/sandbox_isolation.rs`:
    - Tests covering C-06, C-07, C-08, C-09.

### Acceptance Criteria Coverage

| AC#      | Descrição                                                             | Endereçável nesta Fase (F6)? | Notas                                                             |
| :------- | :-------------------------------------------------------------------- | :--------------------------- | :---------------------------------------------------------------- |
| **C-06** | Verificação de capability em todo binding de função nativa            | Sim                          | Implementado via `guarded_native_fn` e verificado em testes.      |
| **C-07** | Capability não autorizada retorna `EngineError::PermissionDenied`     | Sim                          | Verificado para todas as operações protegidas.                    |
| **C-08** | Subsistemas mantêm `ExecutionContext` isolados, com escopos separados | Sim                          | Verificado com dois contextos paralelos manipulando variáveis.    |
| **C-09** | Script em pânico não derruba o host e aciona o fallback               | Sim                          | Verificado com closure panicking capturada por `TrappedExecutor`. |
