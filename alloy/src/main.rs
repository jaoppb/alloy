//! Alloy CLI — v0.1 entry point.
//!
//! Two things work here, matching the v0.1 micro-deliverables
//! (`ROADMAP-IMPLEMENTACAO-V1.md` §3.1):
//!
//! - `alloy` with no arguments prints help and exits cleanly (code 0).
//! - `alloy --script <path>` compiles the file with [`RhaiEngine`], runs it
//!   under the execution-limit sandbox, and prints the returned value.
//!
//! Argument parsing is [`clap`]; failures are the typed [`AlloyError`].
//! Everything downstream — window, network, rendering — is a later phase.

#![forbid(unsafe_code)]

mod error;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use engine::{CapabilitySet, RuntimeEngine};
use rhai_runtime::RhaiEngine;

use crate::error::AlloyError;

/// The infinitely malleable web browser (v0.1 preview).
///
/// With no options, alloy prints this help and exits (nothing to render yet).
#[derive(Debug, Parser)]
#[command(name = "alloy", version, long_about = None)]
struct Cli {
    /// Compile and run a Rhai muscle script, then print its result.
    #[arg(long, value_name = "PATH")]
    script: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("alloy: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<(), AlloyError> {
    let Some(path) = cli.script.as_deref() else {
        let _ = Cli::command().print_help();
        println!();
        return Ok(());
    };
    run_script(path)
}

fn run_script(path: &Path) -> Result<(), AlloyError> {
    let source = std::fs::read_to_string(path).map_err(|source| AlloyError::ScriptRead {
        path: path.to_path_buf(),
        source,
    })?;

    let engine = RhaiEngine::new();
    let mut context = engine.create_context(CapabilitySet::empty())?;
    let value = engine.eval_value(&mut context, &source)?;

    if !value.is_unit() {
        println!("{value}");
    }
    Ok(())
}
