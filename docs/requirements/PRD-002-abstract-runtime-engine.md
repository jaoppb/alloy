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

| Version | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Adapter action                                                                                                                                                                                                                           |
| ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **1**   | Surface frozen at `F1`.                                                                                                                                                                                                                                                                                                                                                                                                                                    | —                                                                                                                                                                                                                                        |
| **2**   | Review response & v0.2 I1. Every name on the port is a validated newtype, not `&str`: `register_native_fn` / `call_function_value` / the `register_fn` / `call_function` sugar take `&FunctionName`; `set_value` / `get_value` / the `set_variable` sugar take `&VariableName`. `SourceLocation` is an `enum` (`LineColumn` / `LineOnly`) over `Line` / `Column`. `EngineError` gains additive `Dom { operation: String, reason: String }` variant.        | Implement the object-safe methods against `&FunctionName` / `&VariableName` (`name.as_str()` for a raw key); the caller builds the newtype. Read a location via `match` on the enum or `line()` / `column()`. Handle `EngineError::Dom`. |
| **3**   | v0.5 Phase EE. `EngineError` gains additive `Subsystem { subsystem: SubsystemName, operation: String, reason: String }`, where `SubsystemName` is a new `#[non_exhaustive]` enum (`Dom` / `Css` / `Graphics` / `Network` / `Window`). `Dom { operation, reason }` is `#[deprecated]` but **not removed** — still constructible, still matchable. `EngineError::dom(op, reason)` now delegates to `EngineError::subsystem(SubsystemName::Dom, op, reason)`. | Match `EngineError::Subsystem { subsystem, .. }` instead of the deprecated `EngineError::Dom { .. }`; a consumer that still matches `Dom` keeps compiling (with a deprecation warning) until the v0.7 schema-4 removal.                  |

### 4.3 v0.2 F6/I1 amendment — `EngineError::Dom`

The scriptable-DOM bridge (roadmap I1) needs a boundary error that is distinct from `Binding` ("bad native-binding name
/ arity"). `EngineError` gains one variant:

```rust
Dom { operation: String, reason: String }
```

raised when a DOM operation invoked from a script fails for a reason other than a missing capability (an invariant
violation, a stale node id, a busy tree). `core/dom`'s own `DomError` is mapped to it in the `core/runtime/rhai`
adapter; `core/dom` never names `EngineError`.

### 4.4 v0.2 F6 amendment — the object-safe `dyn` companion (`ADR-0013`)

`ADR-0011` item 2 requires an object-safe companion for the port. It lands in
`core/engine/src/application/dyn_bridge.rs` as three `dyn`-safe traits and a free function, all speaking only boundary
types:

- `DynExecutionContext` — the object-safe core of `ExecutionContext` verbatim, plus `as_any_mut`.
- `DynCompiledScript` — an erased compiled program.
- `DynRuntimeEngine : Send + Sync` — `create_context_dyn` / `compile_dyn` / `eval_value_dyn` /
  `eval_compiled_value_dyn`, all `-> Result<_, EngineError>`.
- `eval_typed::<T: FromEngineValue>(&dyn DynRuntimeEngine, &mut dyn DynExecutionContext, &str) -> Result<T, EngineError>`.

Blanket impls (`impl<C: ExecutionContext + 'static> DynExecutionContext for C`,
`impl<S: Send + Sync + 'static> DynCompiledScript for S`,
`impl<E: RuntimeEngine> DynRuntimeEngine for E where E::Context: 'static, E::CompiledScript: 'static`) give every
adapter the companion for free. **No existing signature changes**, so this amendment does not move `PORT_SCHEMA_VERSION`
on its own. `engine::conformance::run_dyn_suite` is its conformance form; `MockEngine` and `RhaiEngine` both pass it.

### 4.5 v0.5 Phase EE amendment — `EngineError::Subsystem`

`EngineError::Dom { operation, reason }` (§4.3) named exactly one subsystem. v0.5 adds three more script bridges —
`core/css` (the scriptable cascade adapter, Phase M), `core/network` (the scriptable `RequestPolicy`, Phase M) and
`core/window` (UI/window bindings, Phase M) — and a fourth bespoke variant per subsystem would leave the enum growing
one arm per crate forever, with no bound. `EngineError` instead gains one generalized variant:

```rust
#[non_exhaustive]
pub enum SubsystemName { Dom, Css, Graphics, Network, Window }

Subsystem { subsystem: SubsystemName, operation: String, reason: String }
```

raised when a subsystem operation invoked from a script fails for a reason other than a missing capability — the same
shape `Dom` always was, generalized by an added `subsystem` discriminant. `Dom { operation, reason }` is marked
`#[deprecated]` but **kept, not removed**: the one existing caller (`EngineError::dom`, used by
`core/runtime/rhai-bindings::dom_bindings`) now delegates to `Self::subsystem(SubsystemName::Dom, ..)`, so it produces
`Subsystem` without any call-site changing. `Dom`'s full removal — dropping the deprecated variant — is deferred to a
v0.7 `PORT_SCHEMA_VERSION` 4 change, once `core/js` (the next consumer of this pattern) has landed and the deprecation
window has been open a full release. Naming `Css` / `Graphics` / `Network` / `Window` in `SubsystemName` now, ahead of
Phase M actually raising them, is the same anticipatory-naming precedent `Capability` already set in v0.1
(`NETWORK_LISTEN` and `DEVTOOLS_INSPECT` had no producer for several releases either).

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
