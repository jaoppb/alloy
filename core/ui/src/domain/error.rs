//! Typed errors of the chrome domain.

/// Why the chrome could not be laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UiError {
    /// Surface dimensions were invalid (must be positive and fit the app-unit envelope).
    #[error("invalid surface dimensions: width and height must be positive")]
    InvalidDimensions,
}
