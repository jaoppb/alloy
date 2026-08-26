use thiserror::Error;

/// Domain error enum representing failures in graphics initialization, rasterization, or encoding.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GraphicsError {
    /// File I/O failure.
    #[error("Graphics I/O error: {0}")]
    IoError(String),
    /// Image encoding failure (e.g. PNG compression failure).
    #[error("Graphics encoding error: {0}")]
    EncodingError(String),
    /// Backend initialization failure.
    #[error("Graphics backend init failed: {0}")]
    InitializationFailed(String),
    /// Invalid render command coordinates or dimensions.
    #[error("Invalid render command: {0}")]
    InvalidCommand(String),
}
