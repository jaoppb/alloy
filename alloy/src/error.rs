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

    /// Navigation or a subresource fetch failed (v0.5 Phase I4).
    #[error(transparent)]
    Network(#[from] network::NetworkError),

    /// The native window or its presenter failed (v0.5 Phase I4).
    #[error(transparent)]
    Window(#[from] window::WindowError),

    /// A fetched image's bytes did not decode as PNG (v0.5 Phase I4).
    #[error("could not decode fetched image: {0}")]
    ImageDecode(#[from] graphics::png::PngProblem),

    /// An HTTP fetch completed but the server answered with a non-`2xx` status
    /// (v0.5 Phase I4). Used for both the main document and subresources: an
    /// error-page body must never be handed to `html::parse`, the CSS parser
    /// or the PNG decoder as if it were the resource.
    #[error("{url} returned HTTP {status}")]
    HttpStatus { url: String, status: u16 },

    /// The main document fetch succeeded but the body was empty (a bare `200`
    /// with no content, a `204`, a non-followable `3xx`) — rendering it would
    /// be a silent blank window, so it falls back to the error card instead
    /// (v0.5 Phase I4).
    #[error("{url} returned an empty document body")]
    EmptyDocument { url: String },

    /// The main document fetch succeeded but the body is not valid UTF-8 text
    /// (a missing or non-textual `Content-Type`, so `core/network` left the
    /// bytes untranscoded). `html::parse` needs `&str`; falls back to the
    /// error card rather than rendering nothing (v0.5 Phase I4).
    #[error("{url} returned a document body that is not valid UTF-8 text")]
    NonTextualDocument { url: String },
}
