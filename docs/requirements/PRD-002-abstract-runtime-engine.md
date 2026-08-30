# PRD-002: Abstract Runtime Engine & Script Execution

- **Status**: Accepted
- **Author**: Core Architecture Team
- **Date**: 2026-08-22 (retrofitted to the `ADR-0011` Replaceable Port Contract: 2026-08-30)
- **Target Release**: v0.1.0-alpha
- **Port**: `RuntimeEngine` / `ExecutionContext` — `core/engine`. Freeze point `F1`. Contract record:
  `docs/architecture/runtime-engine-port-contract.md`.

---

## 1. Executive Summary

Alloy requires an abstract execution layer that decouples the Rust domain data models from any specific scripting
interpreter. The primary launch backend is **Rhai** ([rhai.rs](https://rhai.rs/)), but the engine abstraction must
support future backends such as JavaScript (Boa/QuickJS) and WebAssembly without altering domain crates.

---

## 2. Problem Statement

Binding domain crates directly to a specific embedded scripting engine creates tight coupling:

- Type registrations and memory management become language-specific.
- Replacing or testing alternative script engines requires invasive refactors across all crates.
- Sandbox security rules cannot be uniformly enforced across different runtime engines.

### 2.1 Variation model — what may legitimately differ between backends

An implementation of this port (Rhai today; Boa / QuickJS / a Wasm host / a mock later) MAY differ in:

- **Surface language and syntax.** `compile` accepts an opaque `&str`; nothing constrains the grammar.
- **Compiled representation.** `type CompiledScript` is the backend's own parsed form (Rhai: `Arc<rhai::AST>`). It is
  opaque to callers and only required to be `Send + Sync + 'static`.
- **Native-binding mechanism.** How a backend turns a `NativeFn` into something its scripts can call (Rhai:
  `register_raw_fn` with `Dynamic` slots).
- **Type projection.** How an `EngineType` becomes a script-visible type (Rhai: a bridge to `rhai::CustomType`).
- **Limit primitives and their granularity.** Which knobs of `ExecutionLimits` a backend can honour and how precisely
  (Rhai: operation counter + call/expression depth + a wall-clock `on_progress` guard).
- **Performance and memory profile**, and non-observable evaluation order.

An implementation MUST NOT differ in:

- The **meaning and set** of `EngineValue` shapes and `EngineError` variants (one enum, `ADR-0011` item 4).
- **Determinism of `compile`**: the same source yields an equivalent program every time; a syntax error is always
  `EngineError::Compilation` with a source location when the backend has one.
- **Capability semantics**: a context never widens its grant; `CapabilitySet` is carried for the context's whole life.
- The **object-safe method contracts** (`create_context`, `compile`, `eval_value`, `eval_compiled_value`,
  `set_value`/`get_value`, `register_native_fn`, `register_type_erased`, `reset_scope`, `capabilities`).
- **Isolation**: two contexts from one engine share no script-visible state (`PRD-003:78`).
- **Fault containment**: a script or native-binding panic is trapped as `EngineError::ScriptPanic`; the host process
  never aborts (`PRD-003:79`).

The conformance suite (`engine::conformance`, `ADR-0011` item 6) is the executable form of the "MUST NOT differ" list.

### 2.2 Threat model — trusted-but-fallible author

Muscle scripts run under this port are written by **the user customising their own browser** (`PRD-003:21-24`). They are
_trusted_ but _fallible_. The defended-against failures are:

- Infinite loops and runaway recursion → bounded by `ExecutionLimits` (C-04).
- Panics, thrown values, and logic bugs → trapped, mapped to `EngineError`, host survives (C-09).
- Accidental over-reach (a UI script touching the network) → `CapabilitySet` least-privilege,
  `EngineError::PermissionDenied` (C-06/C-07, enforced per binding from `F6`).

**Out of scope for this port**: deliberately hostile code, sandbox-escape attempts, timing/side-channel attacks, and
resource-exhaustion attacks by a malicious author. Arbitrary third-party page JavaScript is a **different** boundary
with a **different** threat model — `core/js` / `PRD-006`, `ADR-0006:63-68`. The capability system here is
defence-in-depth and least-privilege hygiene, not a containment boundary for adversarial code.

---

## 3. Architecture & Trait Specifications

### 3.1 The `RuntimeEngine` Trait Hierarchy

The abstract engine layer resides in `core/engine` and provides:

```rust
pub trait RuntimeEngine: Send + Sync {
    type Context: ExecutionContext;
    type CompiledScript: Send + Sync;
    type Error: std::error::Error + Send + Sync + 'static;

    fn create_context(&self, capabilities: CapabilitySet) -> Result<Self::Context, Self::Error>;
    fn compile(&self, script_source: &str) -> Result<Self::CompiledScript, Self::Error>;
    fn eval<T: FromEngineValue>(&self, context: &mut Self::Context, script: &str) -> Result<T, Self::Error>;
}

pub trait ExecutionContext {
    type Error: std::error::Error + Send + Sync + 'static;

    fn register_type<T: 'static + CustomType>(&mut self) -> Result<(), Self::Error>;
    fn register_fn<F, Args, Ret>(&mut self, name: &str, f: F) -> Result<(), Self::Error>
    where
        F: EngineFunction<Args, Ret>;
    fn set_variable<V: IntoEngineValue>(&mut self, name: &str, value: V) -> Result<(), Self::Error>;
    fn call_function<Ret: FromEngineValue>(
        &mut self,
        name: &str,
        args: &[EngineValue],
    ) -> Result<Ret, Self::Error>;
    fn reset_scope(&mut self) -> Result<(), Self::Error>;
}
```

### 3.2 Rhai Engine Implementation (`core/runtime/rhai`)

The `RhaiEngine` implements `RuntimeEngine`:

- Wraps `rhai::Engine` and `rhai::Scope`.
- Registers domain types via `rhai::CustomType`.
- Enforces strict execution limits (instruction counter limits, recursion depth limits).
- Provides type marshaling between native Rust structs and `rhai::Dynamic`.

---

## 4. Requirements & Invariants

1. **Deterministic Type Conversions**: Domain types implementing `IntoEngineValue` and `FromEngineValue` must safely
   cross the runtime boundary without raw pointer dereferences.
2. **Zero Global State**: Runtime engines and execution contexts must be instantiable per subsystem and per isolate.
3. **Execution Limits**: All script executions must enforce maximum execution instruction steps (preventing infinite
   `while(true)` loops) and memory allocation ceilings.
4. **Transparent Error Mapping**: Script evaluation errors (syntax errors, runtime panics, type mismatches) must be
   mapped to structured Rust errors with line/column metadata.

### 4.1 Known gap — script-defined hook invocation (deferred)

`PRD-001 §5.2` defines a hook lifecycle (`on_init`, `on_event`, `on_process`, `on_reload`) of functions **defined by a
compiled script** and called by the host by name; `PRD-004` ends its flow at "Invoke `on_reload()`". The v0.1 port has
no method that carries a `CompiledScript` into a by-name call, so `ExecutionContext::call_function` currently means only
"invoke a registered native binding". Closing this needs either an added method
(`call_compiled_function(&self, ctx, &CompiledScript, name, args)`) or a compiled AST attached to the context. **This is
a v0.2 decision and a v0.2 amendment to this PRD.** It does not change any signature already frozen at `F1`.

### 4.2 Boundary-schema migrations (`engine::PORT_SCHEMA_VERSION`)

| Version | Change                                                                                                                                                                                                                                                                                                                                                                                                                                   | Adapter action                                                                                                                                            |
| ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **1**   | Surface frozen at `F1`.                                                                                                                                                                                                                                                                                                                                                                                                                  | —                                                                                                                                                         |
| **2**   | Review response. `ExecutionContext::register_native_fn` / `call_function_value` take `&FunctionName` (validated identifier) instead of `&str`; the `register_fn` / `call_function` sugar still takes `&str` and validates, so callers are unaffected. `SourceLocation` is now an `enum` (`LineColumn` / `LineOnly`) over `Line` / `Column` newtypes — the `column == 0` "unknown" sentinel is gone; `column()` returns `Option<Column>`. | Implement the two core methods against `&FunctionName` (`name.as_str()` for a raw key). Read a location via `match` on the enum or `line()` / `column()`. |

---

## 5. Acceptance Criteria

- [x] `RuntimeEngine` and `ExecutionContext` traits defined in `core/engine`. _(with two `ADR-0011`-mandated deviations,
      recorded in §2.1 and `core/engine/src/application/ports.rs`: no associated `type Error` — one `EngineError`; own
      `EngineType` marker instead of `rhai::CustomType`.)_
- [x] `RhaiEngine` implementation in `core/runtime/rhai` passing trait compliance tests. _(`engine::conformance` suite,
      run from `core/runtime/rhai/tests/conformance.rs`.)_
- [ ] Registered Rust domain struct (`DomNode`) readable and mutable from Rhai script. _(v0.1 proves the mechanism with
      `FixtureNode` in `core/runtime/rhai/tests/fixture_node.rs`; the real `core/dom` `DomNode` — roadmap C-03 — lands
      at integration point `I1`, v0.2.)_
- [x] Execution limit test: an infinite loop in Rhai is aborted with `EngineError::ExecutionLimitExceeded`.
      _(`core/runtime/rhai/tests/execution_limits.rs` — operation ceiling **and** wall-clock ceiling.)_
- [x] Trait-mocking test verifying engine can be replaced without modifying domain crates. _(`MockEngine` +
      `evaluate_subject<E: RuntimeEngine>` in `core/engine/tests/mock_engine.rs`.)_
