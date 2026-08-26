use std::fmt;

/// Domain error enum representing failures in graphics initialization, rasterization, or encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphicsError {
    /// File I/O failure.
    IoError(String),
    /// Image encoding failure (e.g. PNG compression failure).
    EncodingError(String),
    /// Backend initialization failure.
    InitializationFailed(String),
    /// Invalid render command coordinates or dimensions.
    InvalidCommand(String),
}

impl fmt::Display for GraphicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "Graphics I/O error: {e}"),
            Self::EncodingError(e) => write!(f, "Graphics encoding error: {e}"),
            Self::InitializationFailed(e) => write!(f, "Graphics backend init failed: {e}"),
            Self::InvalidCommand(e) => write!(f, "Invalid render command: {e}"),
        }
    }
}

impl std::error::Error for GraphicsError {}
