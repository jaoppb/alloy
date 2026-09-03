//! [`FunctionName`] — a validated native-binding name (review comment on
//! `application/ports.rs`). Replaces the raw `&str` that `register_fn` /
//! `call_function` used to pass around.

use core::fmt;

use crate::domain::error::EngineError;
use crate::domain::ident::require_identifier;

/// The name a native function is registered and called under. Non-empty, and a
/// valid script identifier: an ASCII letter or `_` first, then ASCII
/// alphanumerics or `_`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FunctionName(Box<str>);

impl FunctionName {
    /// Validate `raw`, or explain why it is not a usable binding name via
    /// [`EngineError::Binding`].
    pub fn parse(raw: &str) -> Result<Self, EngineError> {
        require_identifier(raw, "function name")?;
        Ok(Self(Box::from(raw)))
    }

    /// The name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for FunctionName {
    type Error = EngineError;

    fn try_from(raw: &str) -> Result<Self, EngineError> {
        Self::parse(raw)
    }
}

impl TryFrom<String> for FunctionName {
    type Error = EngineError;

    fn try_from(raw: String) -> Result<Self, EngineError> {
        Self::parse(&raw)
    }
}

impl AsRef<str> for FunctionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FunctionName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
