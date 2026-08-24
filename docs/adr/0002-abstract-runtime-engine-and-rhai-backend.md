# ADR-0002: Abstract Runtime Engine Trait and Rhai Backend

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-08-22

---

## Context and Problem Statement

Alloy requires user scripting to control browser behavior. We want to start by supporting **Rhai** because of its
lightweight nature, safety, and seamless Rust integration. However, hardcoding Rhai into core domain crates would
prevent future support for other execution runtimes (e.g. QuickJS, Boa, WebAssembly, Lua). How should we design the
boundary between Rust domain data and script engines?

---

## Decision Drivers

- Decouple domain crates (`core/dom`, `core/css`, `core/html`, etc.) from scripting language specifics.
- Support Rhai as the first-class engine without architectural compromise.
- Enable pluggable script engines in future releases.
- Ensure type-safe data conversion across the engine boundary.

---

## Considered Options

- **Option 1**: Define a generic `RuntimeEngine` and `ExecutionContext` trait in `core/engine`, implemented by
  `core/runtime_rhai`.
- **Option 2**: Couple all domain crates directly to `rhai::Engine` and `rhai::Scope`.
- **Option 3**: Message-passing actor model over asynchronous IPC channels.

---

## Decision Outcome

Chosen option: **Option 1**, because:

1. `core/engine` provides a clean trait interface (`RuntimeEngine`, `ExecutionContext`, `EngineValue`, `CustomType`)
   allowing domain crates to remain completely independent of Rhai or any specific interpreter.
2. `core/runtime_rhai` implements these traits, managing Rhai AST compilation, evaluation, and scope binding.
3. In-process trait dispatch provides sub-microsecond invocation speeds, which are required for high-frequency rendering
   and DOM traversal hooks.

### Consequences

- **Positive**:
    - Domain crates stay pure Rust data models with zero external script runtime dependencies.
    - Adding a new runtime (e.g. `core/runtime_wasm` or `core/runtime_js`) requires only implementing `RuntimeEngine`.
- **Negative**:
    - Requires writing generic trait wrappers and type conversion bridges (`IntoEngineValue`, `FromEngineValue`).
