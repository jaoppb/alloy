# SPDD Analysis: Concrete Rhai Runtime Backend (core/runtime/rhai)

## Original Business Requirement

### PRD-002: Abstract Runtime Engine & Script Execution (§3.2, C-02, C-04)

The `RhaiEngine` implements `RuntimeEngine` in `core/runtime/rhai`:

- Wraps `rhai::Engine` and `rhai::Scope`.
- Registers domain types and native host functions.
- Enforces strict execution limits (instruction counter limits, recursion depth limits).
- Provides type marshaling between native Rust structs, `EngineValue`, and `rhai::Dynamic`.

Acceptance Criteria:

- **C-02**: `RhaiEngine` implementation in `core/runtime/rhai` passing trait compliance tests.
- **C-04**: Execution limit test: an infinite loop in Rhai is aborted with `EngineError::ExecutionLimitExceeded`.

---

## Domain Concept Identification

### Existing Concepts (from codebase)

- `RuntimeEngine` (`core/engine`): Trait defining `create_context`, `compile`, and `eval`.
- `ExecutionContext` (`core/engine`): Trait managing active isolate scopes, variables, and function dispatch.
- `CapabilitySet` & `Capability` (`core/engine`): Bitflags defining sandboxed permissions for isolates.
- `EngineValue` (`core/engine`): Dynamic value representation (Null, Bool, Int, Float, String, Array, Object).
- `EngineError` (`core/engine`): Structured error enum, specifically `ExecutionLimitExceeded`, `PermissionDenied`,
  `SyntaxError`, `RuntimeError`, `TypeMismatch`.
- `Identifier` (`core/engine`): Newtype ensuring validated, non-empty variable and function names.
- `alloy` (`alloy/`): Binary crate with CLI to be wired with `alloy --script <path>`.

### New Concepts Required

- `RhaiEngine`: Concrete struct implementing
  `RuntimeEngine<Context = RhaiContext, CompiledScript = rhai::AST, Error = EngineError>`.
- `RhaiContext`: Concrete struct implementing `ExecutionContext`, holding `rhai::Scope` and `CapabilitySet`.
- `ExecutionLimits`: Value object encapsulating safety ceilings (max operations, max expression depth, max string size).
- Dynamic Value Marshaler: Bidirectional mapping between `EngineValue` and `rhai::Dynamic`.

### Key Business Rules

- **Trait Compliance (C-02)**: `RhaiEngine` must be a drop-in implementation of `RuntimeEngine`, capable of substituting
  `MockEngine` without changing domain code.
- **Execution Limits (C-04)**: Infinite loops (e.g. `while true {}`) must be terminated by Rhai's operations counter and
  mapped to `EngineError::ExecutionLimitExceeded`.
- **Fault Trapping (PRD-003:64-70)**: Script errors must never panic or crash the host process; they must be cleanly
  converted into `EngineError`.
- **Stateless Recompilation (PRD-004)**: Compiling a script produces an immutable `rhai::AST` that is `Send + Sync`.

---

## Strategic Approach

### Solution Direction

- Implement Clean Architecture in `core/runtime/rhai`:
    - `src/domain/`: Limits configuration (`ExecutionLimits`) and value marshaling (`marshaling.rs`).
    - `src/application/`: Engine implementation (`RhaiEngine`), Context implementation (`RhaiContext`).
    - `src/lib.rs`: Public facade re-exporting `RhaiEngine`, `RhaiContext`, `ExecutionLimits`.
- Add `rhai = { version = "1.26.0", features = ["sync"] }` to `[workspace.dependencies]` and
  `core/runtime/rhai/Cargo.toml`.
- Wire `rhai-runtime` into the `alloy` binary crate so `alloy --script <path>` executes scripts.

### Key Design Decisions

- **Rhai `sync` Feature**: Enable thread-safe `Send + Sync` data structures for `Engine` and `AST`, enabling
  multi-threaded hot-reload in later phases.
- **Execution Limits Configuration**: Provide defaults (e.g. 100,000 max operations) with builder methods on
  `RhaiEngine` (`with_limits`, `with_max_operations`).
- **Error Translation Layer**: Map Rhai's `EvalAltResult::ErrorTooManyOperations` directly to
  `EngineError::ExecutionLimitExceeded`.

### Alternatives Considered

- _Wrapping Rhai types directly into public API_: Rejeitado categoricamente pelo ADR-0002 para preservar a pureza das
  traits do `core/engine`.
- _Disabling operations limits_: Rejeitado pelo PRD-002:78-79 e critério C-04.

---

## Risk & Gap Analysis

### Requirement Ambiguities

- PRD-002:48 mentions `register_type<T: 'static + CustomType>`. Because `core/engine` does not depend on Rhai, type
  registration is handled via `ExecutionContext` ports or native function bindings.

### Edge Cases

- Division by zero in Rhai: Must map cleanly to `EngineError::RuntimeError`.
- Recursive functions blowing the call stack: Must be caught by Rhai's call stack depth limit.

### Technical Risks

- Performance of marshaling large arrays/maps between `EngineValue` and `rhai::Dynamic`: Mitigated by shallow reference
  extraction where possible.

### Acceptance Criteria Coverage

| AC#      | Descrição                                                                    | Endereçável nesta Fase (F2)? | Notas                                            |
| :------- | :--------------------------------------------------------------------------- | :--------------------------- | :----------------------------------------------- |
| **C-02** | `RhaiEngine` em `core/runtime/rhai` passando testes de conformidade da trait | Sim                          | Entregável central da Fase F2.                   |
| **C-04** | Loop infinito abortado com `EngineError::ExecutionLimitExceeded`             | Sim                          | Testado explicitamente em `rhai_conformance.rs`. |
