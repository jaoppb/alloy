# ADR-0012: Host Objects and Universal Namespacing for the Script Surface

## Status

Accepted

## Date

2026-08-26

## Context

In the initial scripting surface implementation, native Rust functions were registered into the script isolate via a
flat `register_fn(name, f)` method on `ExecutionContext`. This caused critical architectural issues:

1. **Defect D-01**: Registered native functions in `RhaiContext` were only stored in a local `HashMap` and never
   registered with the underlying `rhai::Engine`. Consequently, scripts evaluated via `engine.eval(...)` could not reach
   any registered functions.
2. **Global Namespace Pollution**: Exposing flat names such as `dom_create_element` or `graphics_push_rect` pollutes the
   global scope, creates naming collisions across subsystems, and deviates from web standards where host capabilities
   live strictly under namespaces (`document`, `renderer`, `console`) and prototype/instance methods
   (`Node.appendChild`).
3. **Repeated Security Checks (C-28)**: In a flat model, capabilities had to be manually guarded per function via
   `guarded_native_fn`, scattering security policy across individual bindings instead of governing them at the domain
   container boundary.
4. **Naked Pointer Arithmetic**: DOM nodes were exposed to scripts as primitive integers (`NodeId(u32)` as
   `EngineValue::Int`), allowing scripts to perform arbitrary integer arithmetic on internal arena indices.

## Decision

1. **Neutral HostObject Abstraction**: Introduce `HostObject` in `core/engine/src/domain/host_object.rs`. A `HostObject`
   defines:
    - `name`: Target namespace or class name (`document`, `Node`, `renderer`).
    - `required_capability`: The capability governing the object or namespace.
    - `is_singleton`: Whether the object is a global singleton instance or an instantiable type.
    - `methods`: Collection of instance and static methods with standard `camelCase` identifiers.
    - `properties`: Property getters and setters.

2. **Opaque Entity Handles**: Extend `EngineValue` with `Handle(Arc<dyn Any + Send + Sync>)` to wrap domain entities
   (such as `NodeId`). Scripts manipulate these instances as native custom objects, abolishing exposed integer IDs.

3. **Port Redesign**: Replace `register_fn` on `ExecutionContext` with `register_host_object(HostObject)`. Flat function
   registration in the root global scope is abolished.

4. **Runtime Bridging in Rhai**: In `core/runtime/rhai`, `HostObject` definitions are registered into `rhai::Engine` as
   custom types and singletons with methods and property getters/setters, making them accessible to script evaluation
   (`engine.eval`) in full compliance with ADR-0002.

## Consequences

### Positive

- **Web Standards Alignment**: Scripts interact with standard objects (`document.createElement("div")`,
  `div.appendChild(p)`, `renderer.pushRect(...)`).
- **Defect D-01 Resolved**: Native host objects and methods are registered with the underlying engine, enabling scripts
  executed via `engine.eval` to call host methods seamlessly.
- **Declarative Security**: Capabilities are enforced at the `HostObject` container boundary.
- **Safety**: Entity IDs cannot be forged through integer arithmetic in scripts.

### Negative / Trade-offs

- Requires rewriting existing flat bindings in `core/dom` and `core/graphics`.
- Requires runtime engines to support custom type and method registration.
