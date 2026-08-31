//! The guarded-binding table (**C-06**).
//!
//! [`RhaiContext::register_guarded_binding`][crate::RhaiContext::register_guarded_binding]
//! is the chokepoint; this module gives it a declarative form. A subsystem
//! declares its native bindings as a `&[GuardedBinding]` and installs the whole
//! table in one call — and the F6 conformance sweep walks the same shape.
//!
//! v0.2 ships **no** production guarded bindings (all DOM access is through
//! `NodeHandle` methods, which self-guard). The mechanism is here, tested, and
//! ready for the first scripted policy port.

use engine::{Arity, Capability, EngineError, FunctionName, NativeFn};

use crate::infrastructure::context::RhaiContext;

/// One capability-guarded native binding: a `name`, the fixed `arity` its
/// script-visible signature reserves, the `required` capability, and the
/// `handler` body.
#[derive(Clone)]
pub struct GuardedBinding {
    pub name: &'static str,
    pub arity: Arity,
    pub required: Capability,
    pub handler: NativeFn,
}

impl GuardedBinding {
    #[must_use]
    pub fn new(name: &'static str, arity: Arity, required: Capability, handler: NativeFn) -> Self {
        Self {
            name,
            arity,
            required,
            handler,
        }
    }
}

/// Install every binding in `table` on `context` through the capability guard.
pub fn install_guarded_table(
    context: &mut RhaiContext,
    table: &[GuardedBinding],
) -> Result<(), EngineError> {
    for binding in table {
        let name = FunctionName::parse(binding.name)?;
        context.register_guarded_binding(
            &name,
            binding.arity,
            binding.required,
            binding.handler.clone(),
        )?;
    }
    Ok(())
}
