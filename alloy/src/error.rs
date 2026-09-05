//! [`AlloyError`] — the typed failure of a CLI invocation (review comment on
//! `main.rs:45`). `main` renders it and maps to a non-zero [`ExitCode`].
//!
//! [`ExitCode`]: std::process::ExitCode

use std::io;
use std::path::PathBuf;

use dom::DomError;
use engine::EngineError;

/// Something went wrong running `alloy`.
#[derive(Debug, thiserror::Error)]
pub enum AlloyError {
    /// The `--script` path could not be read.
    #[error("cannot read script {}: {source}", path.display())]
    ScriptRead {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The HTML input file could not be read.
    #[error("cannot read HTML file {}: {source}", path.display())]
    HtmlRead {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The output PNG file could not be written.
    #[error("cannot write output PNG {}: {source}", path.display())]
    OutputWrite {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Compiling or running the script under the sandbox failed.
    #[error(transparent)]
    Engine(#[from] EngineError),

    /// Serializing or operating on the DOM failed.
    #[error("could not serialize the DOM: {0}")]
    Dom(#[from] DomError),

    /// Parsing HTML failed.
    #[error(transparent)]
    Html(#[from] html::HtmlError),

    /// CSS cascade or layout resolution failed.
    #[error(transparent)]
    Css(#[from] css::CssError),

    /// Display list generation or rasterization failed.
    #[error(transparent)]
    Graphics(#[from] graphics::GraphicsError),

    /// Surface dimensions were invalid (must be positive).
    #[error("invalid surface dimensions: width and height must be positive")]
    InvalidDimensions,
}
