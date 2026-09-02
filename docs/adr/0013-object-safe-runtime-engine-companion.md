# ADR-0013: Object-Safe `dyn` Companion for the `RuntimeEngine` Port

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-08-30

---

## Context and Problem Statement

`ADR-0011` (Replaceable Port Contract) item 2 requires every port trait to be object-safe **or** to ship "an object-safe
companion form". The `RuntimeEngine` / `ExecutionContext` port fails the first test:

- `RuntimeEngine::create_context` returns `Self::Context` by value and `compile` returns `Self::CompiledScript` by value
  — associated types by value make the trait non-object-safe.
- The PRD-002 sugar (`eval::<T>`, `register_fn`, `set_variable::<V>`, `call_function::<T>`, `register_type::<T>`) is
  generic; it is already scoped `where Self: Sized`, so it does not itself break object-safety, but it means the
  ergonomic surface is unavailable through a trait object.

`ExecutionContext`'s **required** methods were deliberately kept object-safe at F1 (`dyn ExecutionContext` compiles),
but `RuntimeEngine` has no `dyn` form. v0.1 had no consumer that needed one, so the companion was recorded as a deferral
to v0.2 (`runtime-engine-port-contract.md` §2). v0.2 F6 is where it comes due: the roadmap wires an engine into the
binary and the conformance story wants a single "any engine, through a pointer" entry point.

How should the port expose a `dyn`-dispatchable engine handle without changing any F1-frozen signature?

---

## Decision Drivers

- Satisfy `ADR-0011` item 2 without reopening the F1 freeze (`ADR-0011:99-101`).
- **Purely additive**: introduce new types only; change no existing trait method.
- No generic method on the `dyn` boundary (that is what broke object-safety in the first place).
- Every adapter that passes the core conformance suite must get the companion **for free** and pass a companion suite
  too.
- Keep the boundary vocabulary unchanged: only `EngineValue` / `EngineError` / `CapabilitySet` / `TypeRegistration` /
  `NativeFn` / `&str` cross the `dyn` seam.

---

## Considered Options

- **Option 1** — a hand-written `dyn`-safe companion trio (`DynRuntimeEngine` / `DynExecutionContext` /
  `DynCompiledScript`) plus a free `eval_typed`, with blanket impls that adapt every `RuntimeEngine` /
  `ExecutionContext`.
- **Option 2** — make `RuntimeEngine` itself object-safe by dropping the associated types (return
  `Box<dyn ExecutionContext>` / `Box<dyn Any>` directly) and moving the generic sugar to an extension trait.
- **Option 3** — leave `RuntimeEngine` non-object-safe; consumers always monomorphise (`fn run<E: RuntimeEngine>(…)`).

---

## Decision Outcome

Chosen option: **Option 1**.

Option 2 changes the frozen F1 surface (`create_context` / `compile` return types) and forces a schema bump and a
migration for every existing adapter — exactly what the freeze exists to prevent. Option 3 leaves item 2 permanently
unsatisfied and blocks any future "list of heterogeneous engines" use (`core/js` alongside `rhai-runtime`).

### The companion (`core/engine/src/application/dyn_bridge.rs`)

```text
DynExecutionContext                       // object-safe core of ExecutionContext, verbatim
    capabilities / register_type_erased / register_native_fn / set_value / get_value
    call_function_value / reset_scope
    as_any_mut() -> &mut dyn Any          // for the engine to recover its concrete context

DynCompiledScript
    as_any() -> &dyn Any

DynRuntimeEngine : Send + Sync
    create_context_dyn(CapabilitySet) -> Box<dyn DynExecutionContext>
    compile_dyn(&str)                 -> Box<dyn DynCompiledScript>
    eval_value_dyn(&mut dyn DynExecutionContext, &str)                         -> EngineValue
    eval_compiled_value_dyn(&mut dyn DynExecutionContext, &dyn DynCompiledScript) -> EngineValue

eval_typed::<T: FromEngineValue>(&dyn DynRuntimeEngine, &mut dyn DynExecutionContext, &str) -> T
```

All methods return `Result<_, EngineError>`. Blanket impls provide the companion with **no per-adapter code**:

- `impl<C: ExecutionContext + 'static> DynExecutionContext for C` — trivial delegation; `as_any_mut` returns `self`.
- `impl<S: Send + Sync + 'static> DynCompiledScript for S`.
- `impl<E: RuntimeEngine> DynRuntimeEngine for E where E::Context: 'static, E::CompiledScript: 'static` —
  `eval_value_dyn` downcasts `context.as_any_mut()` back to `E::Context` (a mismatch is `EngineError::Binding`).

No change to `RuntimeEngine` or `ExecutionContext`. The only v0.2 boundary-aggregate change is the separate
`EngineError::Dom` variant (I1, `PRD-002` §4.2), which is what moves `PORT_SCHEMA_VERSION` to `2`.

### Conformance

`engine::conformance::run_dyn_suite(Box<dyn DynRuntimeEngine>)` mirrors `run_core_suite` through the `dyn` API.
`MockEngine` and `RhaiEngine` both run it (`core/engine/tests/mock_engine.rs`,
`core/runtime/rhai/tests/dyn_conformance.rs`). `runtime-engine-port-contract.md` item 2 moves to ✅.

---

## Consequences

- **Positive**:
    - `ADR-0011` item 2 satisfied for this port; `Box<dyn DynRuntimeEngine>` is a usable handle.
    - Zero cost to existing adapters — blanket impls, no signature churn, no schema bump _for the companion_.
    - The `dyn` seam carries no generic method, so it cannot regress object-safety.
- **Negative**:
    - Two parallel surfaces (generic + `dyn`) to keep in sync; the conformance suite is the guard.
    - `eval_value_dyn` does a `downcast` per call — a branch and a `TypeId` compare, not a real cost, but it can fail
      with `EngineError::Binding` if a caller pairs a context and an engine of different concrete types.
- Numbering: **0013**. `ADR-0011:124` reserves **0012** for the content-JS engine selection (`boa` vs alternatives).
