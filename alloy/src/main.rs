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

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Render {
            file,
            output,
            width,
            height,
            css,
        }) => {
            execute_render(file, output, *width, *height, css.as_deref());
        }
        None => {
            if let Some(script_path) = &cli.script {
                execute_script(script_path);
            } else if let Some(target) = &cli.target {
                println!("Opening target: {target}");
            } else {
                println!(
                    "Alloy browser engine v{} initialized.",
                    env!("CARGO_PKG_VERSION")
                );
            }
        }
    }
}

fn execute_render(file: &str, output: &str, width: u32, height: u32, css_path: Option<&str>) {
    let html_content = match std::fs::read_to_string(file) {
        Ok(src) => src,
        Err(err) => {
            eprintln!("Failed to read HTML file '{file}': {err}");
            std::process::exit(1);
        }
    };

    let dom = match parse_html(&html_content) {
        Ok(tree) => tree,
        Err(err) => {
            eprintln!("Failed to parse HTML: {err}");
            std::process::exit(1);
        }
    };

    let stylesheet = if let Some(css_file) = css_path {
        match std::fs::read_to_string(css_file) {
            Ok(css_content) => parse_css(&css_content).unwrap_or_default(),
            Err(err) => {
                eprintln!("Failed to read CSS file '{css_file}': {err}");
                StyleSheet::default()
            }
        }
    } else {
        extract_inline_style(&dom).unwrap_or_default()
    };

    let styled_tree = StyleCascade::build_styled_tree(&dom, &stylesheet);
    let display_list = LayoutEngine::layout(&dom, &styled_tree, width as f32, height as f32);

    let mut backend = GraphicsBackendFactory::create_headless(width, height);
    if let Err(err) = backend.render(&display_list) {
        eprintln!("Graphics rendering failed: {err}");
        std::process::exit(1);
    }

    if let Err(err) = backend.save_png(Path::new(output)) {
        eprintln!("Failed to save PNG image to '{output}': {err}");
        std::process::exit(1);
    }

    println!("Rendered '{file}' -> '{output}' ({width}x{height}) successfully.");
}

fn extract_inline_style(dom: &dom::DomTree) -> Option<StyleSheet> {
    let root = dom.root()?;
    let style_tag = TagName::new("style").ok()?;
    let style_nodes = DomService::find_by_tag_name(dom, root, &style_tag);
    let style_node_id = *style_nodes.first()?;
    let css_text = DomService::get_text_content(dom, style_node_id);
    parse_css(&css_text).ok()
}

fn execute_script(script_path: &str) {
    let source = match std::fs::read_to_string(script_path) {
        Ok(src) => src,
        Err(err) => {
            eprintln!("Failed to read script '{script_path}': {err}");
            std::process::exit(1);
        }
    };

    let engine = RhaiEngine::new();
    let mut context = match engine.create_context(CapabilitySet::all()) {
        Ok(ctx) => ctx,
        Err(err) => {
            eprintln!("Failed to create script context: {err}");
            std::process::exit(1);
        }
    };

    match engine.eval::<EngineValue>(&mut context, &source) {
        Ok(val) => {
            println!("{val:?}");
        }
        Err(err) => {
            eprintln!("Script execution failed: {err}");
            std::process::exit(1);
        }
    }
}
