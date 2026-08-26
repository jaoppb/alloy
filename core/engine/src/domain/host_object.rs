use crate::domain::capability::Capability;
use crate::domain::error::EngineError;
use crate::domain::identifier::Identifier;
use crate::domain::value::EngineValue;
use std::sync::Arc;

/// Type alias for native methods belonging to a `HostObject`.
///
/// Receives:
/// 1. `Option<&EngineValue>` (the target/receiver instance `this`, if invoked on an instance; `None` for static/singleton calls).
/// 2. `&[EngineValue]` (the method arguments).
pub type HostMethodFn = Arc<
    dyn Fn(Option<&EngineValue>, &[EngineValue]) -> Result<EngineValue, EngineError> + Send + Sync,
>;

/// Type alias for native property getter functions.
pub type HostGetterFn =
    Arc<dyn Fn(Option<&EngineValue>) -> Result<EngineValue, EngineError> + Send + Sync>;

/// Type alias for native property setter functions.
pub type HostSetterFn =
    Arc<dyn Fn(Option<&EngineValue>, EngineValue) -> Result<(), EngineError> + Send + Sync>;

/// Declarative representation of a host object, entity type, or namespace exposed to scripts (ADR-0012, N-01).
#[derive(Clone)]
pub struct HostObject {
    name: Identifier,
    required_capability: Option<Capability>,
    is_singleton: bool,
    methods: Vec<(Identifier, HostMethodFn)>,
    properties: Vec<(Identifier, HostGetterFn, Option<HostSetterFn>)>,
}

impl HostObject {
    /// Creates a new `HostObject` definition for a singleton object or entity namespace.
    #[must_use]
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            required_capability: None,
            is_singleton: true,
            methods: Vec::new(),
            properties: Vec::new(),
        }
    }

    /// Sets whether this host object represents a global singleton (e.g. `document`, `renderer`)
    /// or an instantiable type whose methods apply to instance handles (e.g. `Node`).
    #[must_use]
    pub fn with_singleton(mut self, singleton: bool) -> Self {
        self.is_singleton = singleton;
        self
    }

    /// Specifies the security capability required to access this host object.
    #[must_use]
    pub fn with_capability(mut self, capability: Capability) -> Self {
        self.required_capability = Some(capability);
        self
    }

    /// Adds a method to this host object in `camelCase`.
    pub fn add_method(
        &mut self,
        name: Identifier,
        f: impl Fn(Option<&EngineValue>, &[EngineValue]) -> Result<EngineValue, EngineError>
        + Send
        + Sync
        + 'static,
    ) -> &mut Self {
        self.methods.push((name, Arc::new(f)));
        self
    }

    /// Adds a property with getter and optional setter to this host object.
    pub fn add_property(
        &mut self,
        name: Identifier,
        getter: impl Fn(Option<&EngineValue>) -> Result<EngineValue, EngineError>
        + Send
        + Sync
        + 'static,
        setter: Option<HostSetterFn>,
    ) -> &mut Self {
        self.properties.push((name, Arc::new(getter), setter));
        self
    }

    /// Returns the name of the host object or namespace.
    #[must_use]
    pub const fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the required capability, if any.
    #[must_use]
    pub const fn required_capability(&self) -> Option<Capability> {
        self.required_capability
    }

    /// Returns whether this object is a global singleton.
    #[must_use]
    pub const fn is_singleton(&self) -> bool {
        self.is_singleton
    }

    /// Returns the registered methods.
    #[must_use]
    pub fn methods(&self) -> &[(Identifier, HostMethodFn)] {
        &self.methods
    }

    /// Returns the registered properties.
    #[must_use]
    pub fn properties(&self) -> &[(Identifier, HostGetterFn, Option<HostSetterFn>)] {
        &self.properties
    }
}
