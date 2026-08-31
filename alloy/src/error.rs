//! [`AlloyError`] — the typed failure of a CLI invocation (review comment on
//! `main.rs:45`). `main` renders it and maps to a non-zero [`ExitCode`].
//!
//! [`ExitCode`]: std::process::ExitCode

use std::io;
use std::path::PathBuf;

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

    /// Compiling or running the script under the sandbox failed.
    #[error(transparent)]
    Engine(#[from] EngineError),
}
