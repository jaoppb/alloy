#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};
use css::{StyleCascade, StyleSheet, parse_css};
use dom::{DomService, TagName};
use engine::{CapabilitySet, EngineValue, RuntimeEngine};
use graphics::{GraphicsBackendFactory, LayoutEngine};
use html::parse_html;
use rhai_runtime::RhaiEngine;
use std::path::Path;

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

    /// URL or local file path to open
    #[arg(value_name = "TARGET")]
    pub target: Option<String>,
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
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Render {
            file,
            output,
            width,
            height,
            css,
        }) => execute_render(file, output, *width, *height, css.as_deref()),
        None => {
            if let Some(script_path) = &cli.script {
                return execute_script(script_path);
            }
            if let Some(target) = &cli.target {
                println!("Opening target: {target}");
                return Ok(());
            }
            println!(
                "Alloy browser engine v{} initialized.",
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        }
    }
}

fn execute_render(
    file: &str,
    output: &str,
    width: u32,
    height: u32,
    css_path: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let html_content = std::fs::read_to_string(file)
        .map_err(|err| format!("Failed to read HTML file '{file}': {err}"))?;

    let dom = parse_html(&html_content).map_err(|err| format!("Failed to parse HTML: {err}"))?;

    let stylesheet = match css_path {
        Some(css_file) => match std::fs::read_to_string(css_file) {
            Ok(css_content) => parse_css(&css_content).unwrap_or_default(),
            Err(err) => {
                eprintln!("Failed to read CSS file '{css_file}': {err}");
                StyleSheet::default()
            }
        },
        None => extract_inline_style(&dom).unwrap_or_default(),
    };

    let styled_tree = StyleCascade::build_styled_tree(&dom, &stylesheet);
    let display_list = LayoutEngine::layout(&dom, &styled_tree, width as f32, height as f32);

    let mut backend = GraphicsBackendFactory::create_headless(width, height);
    backend
        .render(&display_list)
        .map_err(|err| format!("Graphics rendering failed: {err}"))?;

    backend
        .save_png(Path::new(output))
        .map_err(|err| format!("Failed to save PNG image to '{output}': {err}"))?;

    println!("Rendered '{file}' -> '{output}' ({width}x{height}) successfully.");
    Ok(())
}

fn extract_inline_style(dom: &dom::DomTree) -> Option<StyleSheet> {
    let root = dom.root()?;
    let style_tag = TagName::new("style").ok()?;
    let style_nodes = DomService::find_by_tag_name(dom, root, &style_tag);
    let style_node_id = *style_nodes.first()?;
    let css_text = DomService::get_text_content(dom, style_node_id);
    parse_css(&css_text).ok()
}

fn execute_script(script_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(script_path)
        .map_err(|err| format!("Failed to read script '{script_path}': {err}"))?;

    let engine = RhaiEngine::new();
    let mut context = engine
        .create_context(CapabilitySet::all())
        .map_err(|err| format!("Failed to create script context: {err}"))?;

    let val = engine
        .eval::<EngineValue>(&mut context, &source)
        .map_err(|err| format!("Script execution failed: {err}"))?;

    println!("{val:?}");
    Ok(())
}
