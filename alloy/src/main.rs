#![forbid(unsafe_code)]

use alloy::{AlloyCliError, VERSION_FINGERPRINT, XdgScriptManager, execute_render};
use clap::{Parser, Subcommand};
use engine::{CapabilitySet, EngineValue, RuntimeEngine};
use rhai_runtime::RhaiEngine;
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser, Debug)]
#[command(
    name = "alloy",
    author,
    version,
    about = "The infinitely malleable, fully modular web browser",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to a script to execute directly
    #[arg(short, long, value_name = "SCRIPT")]
    pub script: Option<String>,

    /// Custom directory for script resolution shadowing
    #[arg(long, value_name = "DIR")]
    pub scripts_dir: Option<PathBuf>,

    /// Enable live file watching and automatic origin syncing
    #[arg(short, long)]
    pub watch: bool,

    /// URL or local file path to open
    #[arg(value_name = "TARGET")]
    pub target: Option<String>,

    /// Log format output: text, json, pretty
    #[arg(long, default_value = "text")]
    pub log_format: String,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Render an HTML file directly to an image without opening a window (headless, I2)
    Render {
        /// Input HTML file
        #[arg(value_name = "FILE")]
        file: String,

        /// Output PNG path
        #[arg(short, long, default_value = "output.png")]
        output: String,

        /// Viewport width in pixels
        #[arg(long, default_value_t = 800)]
        width: u32,

        /// Viewport height in pixels
        #[arg(long, default_value_t = 600)]
        height: u32,

        /// Optional external CSS file path
        #[arg(long)]
        css: Option<String>,

        /// Enable live watching and auto-sync of origin scripts
        #[arg(short, long)]
        watch: bool,
    },
}

fn init_tracing(format: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = fmt().with_env_filter(filter);
    match format {
        "json" => subscriber.json().init(),
        "pretty" => subscriber.pretty().init(),
        _ => subscriber.compact().init(),
    }
}

fn main() -> Result<(), AlloyCliError> {
    let cli = Cli::parse();
    init_tracing(&cli.log_format);

    let xdg = XdgScriptManager::new(cli.scripts_dir.clone())?;
    xdg.seed_scripts()?;

    match &cli.command {
        Some(Commands::Render {
            file,
            output,
            width,
            height,
            css,
            watch,
        }) => execute_render(
            file,
            output,
            *width,
            *height,
            css.as_deref(),
            *watch || cli.watch,
            &xdg,
        ),
        None => {
            if let Some(script_path) = &cli.script {
                return execute_script(script_path);
            }
            if let Some(target) = &cli.target {
                info!("Opening target: {target}");
                return Ok(());
            }
            info!(
                "Alloy browser engine v{} initialized with XDG isolation at {:?}",
                VERSION_FINGERPRINT,
                xdg.data_version_dir()
            );
            Ok(())
        }
    }
}

fn execute_script(script_path: &str) -> Result<(), AlloyCliError> {
    let source = std::fs::read_to_string(script_path)?;

    let engine = RhaiEngine::new();
    let mut context = engine.create_context(CapabilitySet::all())?;

    let val = engine
        .eval::<EngineValue>(&mut context, &source)
        .map_err(|err| AlloyCliError::ScriptExecution(err.to_string()))?;

    info!("{val:?}");
    Ok(())
}
