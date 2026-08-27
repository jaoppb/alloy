use thiserror::Error;

/// Strongly typed errors produced during CLI execution (ADR-0011, C-42, C-43).
#[derive(Debug, Error)]
pub enum AlloyCliError {
    /// File or I/O operation error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// HTML parsing failure.
    #[error("Failed to parse HTML: {0}")]
    HtmlParse(String),

    /// CSS parsing failure.
    #[error("Failed to parse CSS: {0}")]
    CssParse(String),

    /// Graphics backend or rendering failure.
    #[error("Graphics rendering failed: {0}")]
    Graphics(#[from] graphics::GraphicsError),

    /// Script engine or execution error.
    #[error("Script error: {0}")]
    Engine(#[from] engine::EngineError),

    /// Script execution failure.
    #[error("Script execution failed: {0}")]
    ScriptExecution(String),

    /// DOM operation error.
    #[error("DOM error: {0}")]
    Dom(#[from] dom::DomError),
}
