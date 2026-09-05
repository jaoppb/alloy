//! Alloy CLI — entry point.
//!
//! Features:
//! - `alloy` with no arguments prints help and exits cleanly (code 0).
//! - `alloy --script <path>` compiles the file with [`RhaiEngine`], runs it
//!   under the execution-limit sandbox with a bound DOM tree (`DOM_READ | DOM_MUTATE`),
//!   logs any non-unit return value via [`tracing`], and prints serialized HTML.
//! - `alloy render <file.html> -o <out.png> [--width W] [--height H]` renders
//!   HTML directly to a PNG file using the headless pipeline.
//! - `alloy <url>` opens a native window and renders the page at `url`
//!   (v0.5 Phase I4) — real network I/O and a real display, so this path has
//!   no automated coverage; the golden e2e suite (`alloy/tests/e2e_golden.rs`)
//!   exercises the same [`alloy::run_browser`] loop over mocks instead.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use alloy::error::AlloyError;
use alloy::logging;
use alloy::{RenderOptions, initial_window_attributes, render_html_to_png, run_browser};
use clap::{Args, CommandFactory, Parser, Subcommand};
use dom::serialize_html;
use engine::profiles;
use network::{AllowAllPolicy, HttpTransport, RealHttpTransport, RequestPolicy, Url};
use rhai_bindings::run_dom_with_fallback;
use rhai_runtime::RhaiEngine;
use window::{SoftbufferPresenter, WindowSystem, WinitSystem};

/// The embedded default DOM script (C-09 fallback): built into the binary so a
/// muscle-script failure always has something to fall back to.
const DEFAULT_DOM_SCRIPT: &str = include_str!("../../scripts/default_dom.rhai");

/// The infinitely malleable web browser.
#[derive(Debug, Parser)]
#[command(name = "alloy", version, long_about = None)]
struct Cli {
    /// Compile and run a Rhai muscle script, then log its result.
    #[arg(long, value_name = "PATH")]
    script: Option<PathBuf>,

    /// Open a native window and render this page (v0.5 Phase I4).
    #[arg(value_name = "URL")]
    url: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Render an HTML file to a PNG image.
    Render(RenderArgs),
}

#[derive(Debug, Args)]
struct RenderArgs {
    /// Path to the input HTML file.
    #[arg(value_name = "FILE")]
    file: PathBuf,

    /// Output PNG path.
    #[arg(short = 'o', long = "output", value_name = "OUT")]
    output: PathBuf,

    /// Viewport width in pixels.
    #[arg(long, value_name = "W", default_value_t = RenderOptions::DEFAULT_WIDTH)]
    width: u32,

    /// Viewport height in pixels.
    #[arg(long, value_name = "H", default_value_t = RenderOptions::DEFAULT_HEIGHT)]
    height: u32,
}

fn main() -> ExitCode {
    logging::init();
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "alloy exited with an error");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<(), AlloyError> {
    if let Some(Commands::Render(args)) = &cli.command {
        return run_render_command(args);
    }
    if let Some(raw_url) = cli.url.as_deref() {
        return run_browse_command(raw_url);
    }
    let Some(path) = cli.script.as_deref() else {
        let _ = Cli::command().print_help();
        return Ok(());
    };
    run_script(path)
}

/// `alloy <url>`: a real window, real network I/O, `AllowAllPolicy` (the
/// scriptable `RequestPolicy` of v0.5 Phase M is a `rhai-bindings` concern,
/// not this binary's default). Runs until the window closes.
fn run_browse_command(raw_url: &str) -> Result<(), AlloyError> {
    let url = Url::parse(raw_url)?;
    let transport: Arc<dyn HttpTransport> = Arc::new(RealHttpTransport::new()?);
    let policy: Arc<dyn RequestPolicy> = Arc::new(AllowAllPolicy::new());

    let mut system = WinitSystem::new()?;
    let attributes = initial_window_attributes()?;
    system.create_window(&attributes)?;
    let window_id_and_handle = system
        .window_handle()
        .map(|handle| (window::WindowId::from_raw(u64::from(handle.id())), handle));
    let Some((window_id, handle)) = window_id_and_handle else {
        return Err(AlloyError::from(window::WindowError::creation_failed(
            "no window handle after create_window succeeded",
        )));
    };
    let mut presenter = SoftbufferPresenter::new(window_id, handle)?;

    let stats = run_browser(
        &url,
        transport,
        policy,
        &mut system,
        &mut presenter,
        attributes.initial_size(),
    )?;
    tracing::info!(relayouts = stats.relayouts, "browser session ended");
    Ok(())
}

fn run_render_command(args: &RenderArgs) -> Result<(), AlloyError> {
    let html = std::fs::read_to_string(&args.file).map_err(|source| AlloyError::HtmlRead {
        path: args.file.clone(),
        source,
    })?;

    let options = RenderOptions::new(args.width, args.height);
    let png_bytes = render_html_to_png(&html, &options)?;

    std::fs::write(&args.output, &png_bytes).map_err(|source| AlloyError::OutputWrite {
        path: args.output.clone(),
        source,
    })?;

    tracing::info!(
        path = %args.output.display(),
        bytes = png_bytes.len(),
        "rendered HTML to PNG"
    );

    Ok(())
}

fn run_script(path: &Path) -> Result<(), AlloyError> {
    let source = std::fs::read_to_string(path).map_err(|source| AlloyError::ScriptRead {
        path: path.to_path_buf(),
        source,
    })?;

    let engine = RhaiEngine::new();
    let (tree, value) = run_dom_with_fallback(
        &engine,
        profiles::dom_parser(),
        &source,
        Some(path),
        DEFAULT_DOM_SCRIPT,
    );

    if let Some(value) = value.filter(|value| !value.is_unit()) {
        tracing::info!(%value, "script result");
    }

    let html = serialize_html(&tree, tree.document())?;
    if !html.is_empty() {
        println!("{html}");
    }
    Ok(())
}
