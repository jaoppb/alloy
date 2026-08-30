# `RuntimeEngine` port — ADR-0011 contract record

The `RuntimeEngine` / `ExecutionContext` seam in `core/engine` is a **Replaceable Subsystem Port** under `ADR-0011`.
This document is its contract record: the state of all seven mandatory items at the `F1` freeze point.

| Item | Contract requirement                                                      | State                                                                                                                                                                                                                                                         |
| ---- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | Seam PRD with variation + threat model                                    | ✅ `PRD-002` §2.1 (variation model), §2.2 (threat model)                                                                                                                                                                                                      |
| 2    | Port traits: assoc types only, no adapter types, object-safe or companion | 🟡 `ExecutionContext` is object-safe now; `RuntimeEngine` is **not** and the `dyn` companion is deferred to v0.2 / ADR-0013 — see §2 below                                                                                                                    |
| 3    | Boundary aggregates: domain-owned, `#[non_exhaustive]`, schema version    | ✅ `EngineValue`, `ValueKind`, `EngineError`, `TypeRegistration` are `#[non_exhaustive]`; `engine::PORT_SCHEMA_VERSION` is the single version knob. **`= 2`** since v0.2 F6/I1 added `EngineError::Dom` (additive; `PRD-002` §4.2)                            |
| 4    | Exactly one typed error, source location                                  | ✅ `EngineError`; `SourceLocation` on `Compilation` / `ScriptRuntime`. v0.2 added `Dom { operation, reason }` for DOM-binding failures, kept distinct from `Binding`                                                                                          |
| 5    | Written lifecycle & concurrency contract                                  | ✅ §5 below                                                                                                                                                                                                                                                   |
| 6    | Conformance suite + reference adapter + `no-<adapter>`                    | ✅ `engine::conformance`; `MockEngine` reference adapter (`core/engine/tests/`); CI `no-engine` job proves `engine`'s graph links no interpreter. `core/dom` (v0.2 F3) links no engine at all — locked by CI asserting `cargo tree -p dom` is dependency-free |
| 7    | Frozen-API milestone                                                      | 🟡 Frozen at `F1` **except** the item-2 companion; `PORT_SCHEMA_VERSION = 2` is that surface (v0.2 F6/I1 bumped it for `EngineError::Dom`; `PRD-002` §4.2). Any boundary change bumps it + adds a `PRD-002` migration note                                    |

---

## 2. Object-safety and the `dyn` companion (item 2)

The port is split in two on purpose:

- An **object-safe required core** whose every method speaks only `EngineValue`, `EngineError`, `CapabilitySet`,
  `TypeRegistration`, `NativeFn`, `&str`. `dyn ExecutionContext` compiles today.
- **Provided sugar** (`eval::<T>`, `register_fn`, `set_variable::<V>`, `call_function::<T>`, `register_type::<T>`)
  carrying `where Self: Sized`, so it never breaks object-safety of the core it layers on.

`RuntimeEngine` itself is **not** object-safe: `create_context` returns `Self::Context` by value and `compile` returns
`Self::CompiledScript` by value. A `dyn`-dispatch companion (`Box<dyn DynRuntimeEngine>` returning
`Box<dyn ExecutionContext>` and an erased compiled handle) is the documented follow-up — **ADR-0013**, roadmap v0.2.
Until it exists, consumers monomorphise: `fn run<E: RuntimeEngine>(engine: &E, …)`.

This is the one item not fully satisfied at `F1`. It is a deliberate, recorded deferral, not an oversight: no consumer
in v0.1 needs `dyn RuntimeEngine`, and adding the companion later is purely additive (it introduces new types, changes
no existing signature).

---

## 5. Lifecycle and concurrency contract (item 5)

### 5.1 Ownership of durable state

**The Skeleton (Rust) owns all durable state** (`ADR-0003`). An `ExecutionContext` holds only _script-local_ state: the
scope's variables, the set of registered native bindings, the set of registered type names. Nothing the host needs to
survive a reload or a fault lives in the context. `reset_scope` clears the scope's variables and keeps the
registrations; it is the only supported way to discard script-local state without dropping the context.

### 5.2 Threading model

- `RuntimeEngine: Send + Sync`. One engine value may be shared across threads and hand out a context per subsystem / per
  isolate (`PRD-002` invariant 2, "zero global state").
- `ExecutionContext` is **not** required to be `Send` or `Sync`. A context is owned by one subsystem on one thread; it
  is not moved between threads mid-life and is never shared. The Rhai adapter's context is in fact `Send` (the `sync`
  feature) but the port does not promise this and consumers must not rely on it.
- `compile` is `&self` and may be called concurrently. `eval_value` / `eval_compiled_value` / every `ExecutionContext`
  mutator takes `&mut` on the context, so a single context is single-threaded by construction.
- Adapters MUST NOT spawn an internal engine thread and MUST NOT block the caller waiting on external work. Evaluation
  is synchronous and returns.

### 5.3 Re-entrancy and suspension

- v0.1 evaluation is **not re-entrant**: a native binding invoked during `eval_*` must not call back into the same
  context's `eval_*` (it holds `&mut`). The type system enforces this.
- There is **no suspend/resume** point in this port. The suspendable-parser handshake that synchronous `<script>` needs
  is a **different** seam (`TokenSink` / `PRD-008`), by design.

### 5.4 Cancellation

- The only cancellation is a **resource-limit breach** (§5.5): the adapter aborts the current evaluation and returns
  `EngineError::ExecutionLimitExceeded`. There is no cooperative cancel token and no "stop this running script from
  another thread" API in v0.1.
- A trapped panic (§5.6) also ends the current evaluation; the context remains usable afterward (proven by
  `core/runtime/rhai/tests/fault_isolation.rs`).

### 5.5 Resource ceilings

`ExecutionLimits` (engine-agnostic, in `core/engine/domain`) carries four knobs: `max_operations`, `max_call_depth`,
`max_expression_depth`, `max_duration`. `ExecutionLimits::strict()` is the default. An adapter maps them onto whatever
its backend provides and reports a breach as `EngineError::ExecutionLimitExceeded { limit }` with the matching
`ExecutionLimit` discriminant. The Rhai adapter maps:

| `ExecutionLimit`  | Rhai mechanism                                                        |
| ----------------- | --------------------------------------------------------------------- |
| `Operations`      | `Engine::set_max_operations` → `ErrorTooManyOperations`               |
| `CallDepth`       | `Engine::set_max_call_levels` → `ErrorStackOverflow`                  |
| `ExpressionDepth` | `Engine::set_max_expr_depths` (parse-time)                            |
| `Duration`        | `Engine::on_progress` reading a per-eval deadline → `ErrorTerminated` |

Memory ceilings (`PRD-002` invariant 3) are **not** enforced in v0.1 — Rhai exposes `set_max_*` for strings/arrays/maps
but wiring them is deferred; the operation and duration ceilings already bound the `while(true)` case (C-04).

### 5.6 Fault behaviour

Follows the trapping / fallback model of `PRD-003:62-70`:

1. **Trapped execution** — a script error is a `Result::Err`; a script or native-binding **panic** is caught
   (`std::panic::catch_unwind`) and returned as `EngineError::ScriptPanic`. The host process never aborts. No Cargo
   profile may set `panic = "abort"`.
2. **Error logging** — reporting the failure to the DevTools event bus is the host's job, not the port's. The `devtools`
   crate is a stub in v0.1; wiring is `F6` (fallback handler) / `F11` (hot-reload diagnostics).
3. **Default fallback** — falling back to a built-in Rust implementation is the _subsystem's_ responsibility (`F6`); the
   port only guarantees the failure is delivered as a typed `EngineError` and the context stays usable.
4. **Non-corrupting scope reset** — `reset_scope` is available after a fault to re-initialise script-local state
   cleanly.

### 5.7 Hot-reload readiness (`ADR-0005`, deferred to `F11`)

The port already provides the pieces an atomic reload needs:

- `compile` returns `Self::CompiledScript`, required to be `Send + Sync + 'static` — the Rhai adapter's is
  `RhaiCompiledScript(Arc<rhai::AST>)`, exactly the shape swapped by an `ArcSwap` / `RwLock<Arc<_>>` holder.
- `eval_compiled_value` runs an already-compiled program, so a running subsystem never re-parses.
- `reset_scope` performs the scope discard step of the reload flow.

Not yet built (all `F11`, structurally additive): the filesystem watcher, the live-script holder that owns the swappable
`Arc`, and — see `PRD-002 §4.1` — invoking the script-defined `on_reload()` hook.

---

## Audit

Re-run `cargo test -p engine -p rhai-runtime` (the conformance suite is item 6), `cargo tree -p engine` (must be
`bitflags` only — item 2/6), and check `engine::PORT_SCHEMA_VERSION` against the last recorded value when reviewing any
change to `EngineValue` / `EngineError` / a trait signature (items 3/7).
