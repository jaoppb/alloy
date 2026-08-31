//! Shared validation for the identifier value objects ([`FunctionName`],
//! [`VariableName`]).
//!
//! [`FunctionName`]: crate::domain::function_name::FunctionName
//! [`VariableName`]: crate::domain::variable_name::VariableName

use crate::domain::error::EngineError;

/// `true` when `raw` is a script identifier: non-empty, an ASCII letter or `_`
/// first, then ASCII alphanumerics or `_`.
#[must_use]
pub fn is_identifier(raw: &str) -> bool {
    let mut characters = raw.chars();
    let starts_valid = characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_');
    starts_valid
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

/// [`is_identifier`], or an [`EngineError::Binding`] naming `kind`.
pub fn require_identifier(raw: &str, kind: &str) -> Result<(), EngineError> {
    if is_identifier(raw) {
        return Ok(());
    }
    Err(EngineError::binding(format!(
        "`{raw}` is not a valid {kind} (identifier: letter or `_`, then letters/digits/`_`)"
    )))
}
