# PRD-006: Web Content JavaScript Runtime Port

- **Status**: Proposed
- **Author**: Core Architecture Team
- **Date**: 2026-08-28
- **Target Release**: v0.7

---

## 1. Executive Summary

`core/js` executes untrusted JavaScript found in page `<script>` tags and modules. The first embedded engine is
`boa_engine`, but the engine must be replaceable (with QuickJS, V8, or a future in-house engine) **without modifying**
`core/dom`, `core/html`, or any consumer. To achieve this, `core/js` defines its own ports in `application/`, and the
concrete engine is an adapter in `infrastructure/`. This port is **distinct** from the `RuntimeEngine` trait of
`PRD-002`: `ADR-0006:63-68` keeps web-content scripting and browser-muscle scripting as separate responsibilities, and
this PRD conforms to the Replaceable Port Contract of `ADR-0011`.

---

## 2. Problem Statement

Binding `core/dom` and `core/html` directly to `boa_engine` types would recreate exactly the coupling that `ADR-0002`
removed for the muscle engine:

1. The two script boundaries have **different threat models**. Muscle scripts are written by the user and are trusted
   (`PRD-003:21-24`); page JavaScript is arbitrary third-party code, potentially adversarial, running on every
   navigation.
2. The two have **different runtime shapes**. `RuntimeEngine` models `create_context` / `compile` / `eval`; web content
   needs realms per tab, `Origin`, a microtask queue, and a host-driven event loop — none of which `RuntimeEngine`
   expresses (`PRD-002:31-59`).
3. `core/dom` must dispatch events into `core/js` while `core/js` reads and mutates `core/dom` — a crate cycle that must
   be broken by dependency inversion, not a shared utility crate.

---

## 3. Architecture & Port Specifications

### 3.1 `ContentScriptEngine` trait (`js/application/ports.rs`)

```rust
pub trait ContentScriptEngine: Send + Sync {
    type Realm;
    type CompiledUnit: Send + Sync;
    type Error: std::error::Error + Send + Sync + 'static;

    fn create_realm(&self, origin: Origin, capabilities: CapabilitySet)
        -> Result<Self::Realm, Self::Error>;
    fn compile(&self, realm: &Self::Realm, source: &str, kind: ScriptKind)
        -> Result<Self::CompiledUnit, Self::Error>;
    fn evaluate(&self, realm: &mut Self::Realm, unit: &Self::CompiledUnit)
        -> Result<JsValue, Self::Error>;
    fn has_pending_work(&self, realm: &Self::Realm) -> bool;
    fn run_one_task(&self, realm: &mut Self::Realm) -> Result<(), Self::Error>;
    fn drain_microtasks(&self, realm: &mut Self::Realm) -> Result<(), Self::Error>;
}
```

`ScriptKind` is `Classic` or `Module`. No `boa` type appears in any signature. The trait carries no generic method, so
it stays object-safe.

### 3.2 `HostBindings` port (`dom/application/ports.rs`, implemented by `core/js`)

The dependency inversion that breaks the `dom` ↔ `js` cycle. `core/dom` defines the trait; `core/js` implements it and
hands it to the engine adapter. It exposes only the DOM surface page script may touch, and every call is scoped to the
realm's own document subtree:

```rust
pub trait HostBindings {
    fn get_node(&self, id: NodeId) -> Option<NodeHandle>;
    fn text_content(&self, node: NodeHandle) -> Option<Text>;
    fn set_text_content(&mut self, node: NodeHandle, value: Text) -> Result<(), DomBindingError>;
    fn set_attribute(&mut self, node: NodeHandle, name: AttrName, value: AttrValue)
        -> Result<(), DomBindingError>;
    fn append_child(&mut self, parent: NodeHandle, child: NodeHandle) -> Result<(), DomBindingError>;
    fn add_event_listener(&mut self, node: NodeHandle, kind: EventKind, listener: ListenerHandle)
        -> Result<(), DomBindingError>;
}
```

### 3.3 `Origin` value object

`Origin` is a newtype triple (`Scheme`, `Host`, `Port`) carried on every `Realm`. `Origin::same_origin(&self, other)` is
the only same-origin check. This value object is also the input to the `WEB_CONTENT` capability profile added to
`PRD-003` in phase `F7`.

### 3.4 Event loop ownership

The host owns the single event loop (roadmap integration point `I5`: one loop owns the main thread). The engine adapter
exposes `has_pending_work` / `run_one_task` and MUST NOT spawn an internal engine thread or block the caller.

### 3.5 `JsValue` marshaling

`JsValue` is an engine-agnostic enum: it mirrors `EngineValue` (`PRD-002`) for primitives and adds
`Function(FunctionHandle)`, `Promise(PromiseHandle)`, `Object(ObjectHandle)`. Handles are opaque; the adapter keeps the
matching GC root alive for the handle's lifetime and drops it on `Drop`.

### 3.6 Resource limits

Each task runs under a **step budget and a heap ceiling distinct from muscle-script limits** (`PRD-002:78`). Exceeding
either aborts the task with `Error::TaskBudgetExceeded` and does not affect other realms.

---

## 4. Requirements & Invariants

1. **No foreign types**: no `boa_engine` (or other adapter) type appears in `js/domain/`, in `HostBindings`, or in any
   public signature of `core/js`.
2. **Least privilege**: a content realm is created with at most `DOM_READ | DOM_MUTATE`, scoped to its own tab. A script
   of one `Origin` cannot reach the DOM or realm state of another (`PRD-003:24`).
3. **Fault isolation**: an engine panic or uncaught exception is trapped as `Error`, is reported to DevTools, and never
   aborts the host process (`PRD-003:79`).
4. **Determinism**: given identical inputs and identical task ordering, `evaluate` plus microtask draining produces
   identical DOM mutations.
5. **Contract compliance**: this port satisfies all seven items of `ADR-0011`, including the `no-boa` feature and the
   `js-conformance` target.

---

## 5. Acceptance Criteria

- [ ] `ContentScriptEngine`, `HostBindings`, `Origin`, `JsValue`, and `ScriptKind` defined in `core/js` / `core/dom`,
      frozen at integration point `I3`.
- [ ] `boa_engine` adapter in `js/infrastructure/` passes the `js-conformance` suite.
- [ ] A second, mock `ContentScriptEngine` swaps in and drives a page test **without changing** `core/dom` or
      `core/html`.
- [ ] A script whose `Origin` differs from a target node's document is denied with `DomBindingError` (cross-origin).
- [ ] `document.getElementById(id).textContent = "x"` through the `boa` adapter mutates the Rust DOM and triggers a
      repaint.
- [ ] A `while (true) {}` task aborts with `Error::TaskBudgetExceeded` without stalling other realms.
- [ ] `core/js` domain and application layers build and test with `--no-default-features` (feature `no-boa`).
- [ ] The `test262` subset pass rate is computed and published per release, and is monotonic across releases
      (`roadmap §5`).
