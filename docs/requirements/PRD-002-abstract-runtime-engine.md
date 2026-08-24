# PRD-002: Abstract Runtime Engine & Script Execution

- **Status**: Accepted
- **Author**: Core Architecture Team
- **Date**: 2026-08-22
- **Target Release**: v0.1.0-alpha

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

---

## 5. Acceptance Criteria

- [ ] `RuntimeEngine` and `ExecutionContext` traits defined in `core/engine`.
- [ ] `RhaiEngine` implementation in `core/runtime/rhai` passing trait compliance tests.
- [ ] Registered Rust domain struct (`DomNode`) readable and mutable from Rhai script.
- [ ] Execution limit test: an infinite loop in Rhai is aborted with `EngineError::ExecutionLimitExceeded`.
- [ ] Trait-mocking test verifying engine can be replaced without modifying domain crates.
