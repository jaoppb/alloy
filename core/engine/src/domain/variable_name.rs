//! [`VariableName`] — a validated scope-variable name (review comment on
//! `application/ports.rs`). The [`FunctionName`] treatment applied to the `name`
//! of `set_value` / `get_value` / `set_variable`.
//!
//! [`FunctionName`]: crate::domain::function_name::FunctionName

use core::fmt;

use crate::domain::error::EngineError;
use crate::domain::ident::require_identifier;

/// The name a value is bound to in a script scope. Same rule as
/// [`FunctionName`](crate::domain::function_name::FunctionName): non-empty, an
/// ASCII letter or `_` first, then ASCII alphanumerics or `_`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VariableName(Box<str>);

impl VariableName {
    /// Validate `raw`, or explain why it is not a usable variable name via
    /// [`EngineError::Binding`].
    pub fn parse(raw: &str) -> Result<Self, EngineError> {
        require_identifier(raw, "variable name")?;
        Ok(Self(Box::from(raw)))
    }

    /// The name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for VariableName {
    type Error = EngineError;

    fn try_from(raw: &str) -> Result<Self, EngineError> {
        Self::parse(raw)
    }
}

impl TryFrom<String> for VariableName {
    type Error = EngineError;

    fn try_from(raw: String) -> Result<Self, EngineError> {
        Self::parse(&raw)
    }
}

impl AsRef<str> for VariableName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VariableName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
