//! Alloy CLI — v0.1 entry point.
//!
//! Two things work here, matching the v0.1 micro-deliverables
//! (`ROADMAP-IMPLEMENTACAO-V1.md` §3.1):
//!
//! - `alloy` with no arguments opens and exits cleanly (code 0).
//! - `alloy --script <path>` compiles the file with [`RhaiEngine`], runs it
//!   under the execution-limit sandbox, and prints the returned value.
//!
//! Argument parsing is hand-rolled: no dependency for two flags (decision 2.8).
//! Everything downstream — window, network, rendering — is a later phase.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use engine::{CapabilitySet, RuntimeEngine};
use rhai_runtime::RhaiEngine;

const USAGE: &str = "\
alloy — the infinitely malleable web browser (v0.1 preview)

USAGE:
    alloy [OPTIONS]

OPTIONS:
    --script <PATH>    Compile and run a Rhai muscle script, then print its result
    -h, --help        Print this help
    -V, --version     Print version

With no options, alloy starts and exits immediately (nothing to render yet).";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match run(&arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("alloy: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &[String]) -> Result<(), String> {
    match Command::parse(arguments)? {
        Command::Idle | Command::Help => {
            println!("{USAGE}");
            Ok(())
        }
        Command::Version => {
            println!("alloy {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::RunScript(path) => run_script(&path),
    }
}

enum Command {
    /// No arguments — open and exit.
    Idle,
    Help,
    Version,
    RunScript(PathBuf),
}

impl Command {
    fn parse(arguments: &[String]) -> Result<Self, String> {
        match arguments.first().map(String::as_str) {
            None => Ok(Self::Idle),
            Some("-h" | "--help") => Ok(Self::Help),
            Some("-V" | "--version") => Ok(Self::Version),
            Some("--script") => parse_script_path(arguments.get(1)),
            Some(other) => Err(format!("unknown argument `{other}` (try `alloy --help`)")),
        }
    }
}

fn parse_script_path(value: Option<&String>) -> Result<Command, String> {
    let path = value.ok_or_else(|| "`--script` needs a file path".to_owned())?;
    Ok(Command::RunScript(PathBuf::from(path)))
}

fn run_script(path: &Path) -> Result<(), String> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;

    let engine = RhaiEngine::new();
    let mut context = engine
        .create_context(CapabilitySet::empty())
        .map_err(|error| error.to_string())?;
    let value = engine
        .eval_value(&mut context, &source)
        .map_err(|error| error.to_string())?;

    if !value.is_unit() {
        println!("{value}");
    }
    Ok(())
}
