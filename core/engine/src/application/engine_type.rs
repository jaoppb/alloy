//! [`EngineType`] — Alloy's own replacement for `rhai::CustomType` in the port
//! surface.
//!
//! PRD-002:48 writes `register_type<T: 'static + CustomType>`, where `CustomType`
//! is a `rhai` trait. Naming it here would drag `rhai` into `core/engine` and
//! collapse the very boundary this crate exists to hold (ADR-0002:49,
//! ADR-0011 item 2). Instead the port speaks [`EngineType`]: a backend-neutral
//! marker producing a [`TypeRegistration`] descriptor. `core/runtime/rhai`
//! bridges `EngineType` to `rhai::CustomType` inside its `infrastructure/`
//! layer, where an adapter dependency is expected.
//!
//! v0.1 carries only the type's script-visible name. Field and method exposure
//! is added at roadmap I1 (v0.2), when the first real domain type — `DomNode` —
//! is registered.

/// A Rust type that may be projected into a script engine's type system so
/// scripts can hold and pass values of it.
pub trait EngineType: 'static {
    /// Describe this type to an engine. Called by the provided
    /// [`ExecutionContext::register_type`][crate::ExecutionContext::register_type].
    fn registration() -> TypeRegistration
    where
        Self: Sized;
}

/// A backend-neutral description of a registrable type. `#[non_exhaustive]`: it
/// gains fields (accessors, methods) at I1 without breaking adapters
/// (ADR-0011 item 3).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct TypeRegistration {
    script_name: &'static str,
}

impl TypeRegistration {
    /// Register under `script_name` — the identifier scripts use for the type.
    #[must_use]
    pub const fn new(script_name: &'static str) -> Self {
        Self { script_name }
    }

    #[must_use]
    pub const fn script_name(&self) -> &'static str {
        self.script_name
    }
}
