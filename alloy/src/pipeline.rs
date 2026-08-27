//! Full headless rendering pipeline orchestration (PRD-001, PRD-005, ADR-0010, I2).

use crate::error::AlloyCliError;
use crate::xdg_scripts::XdgScriptManager;
use css::{StyleCascade, StyleSheet, parse_css};
use dom::{DomHostModule, DomService, DomTree, TagName};
use engine::{
    CapabilitySet, DebounceDuration, EngineValue, ExecutionContext, HostModule, Identifier,
    RuntimeEngine, create_default_script_watcher,
};
use graphics::{
    GraphicsBackendFactory, GraphicsHostModule, LayoutEngine, ScriptDisplayListContainer,
};
use html::parse_html;
use rhai_runtime::RhaiEngine;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

/// Embedded default Rhai script for full pipeline orchestration.
pub const DEFAULT_PIPELINE_SCRIPT: &str = include_str!("pipeline.rhai");

/// Executes the headless render pipeline with optional live file watching.
///
/// # Errors
/// Returns `AlloyCliError` if file reading, parsing, or rendering fails.
pub fn execute_render(
    file: &str,
    output: &str,
    width: u32,
    height: u32,
    css_path: Option<&str>,
    watch_mode: bool,
    xdg: &XdgScriptManager,
) -> Result<(), AlloyCliError> {
    render_frame(file, output, width, height, css_path, xdg)?;

    if watch_mode {
        info!("Starting live watch and origin syncing mode...");
        let watch_paths = xdg.discover_watch_paths();
        info!(
            "Watching {} directories for Rhai modifications.",
            watch_paths.len()
        );

        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = create_default_script_watcher(DebounceDuration::default_50ms());

        for wp in &watch_paths {
            let tx_clone = tx.clone();
            if let Err(e) = watcher.watch(wp, move |path| {
                let _ = tx_clone.send(path);
            }) {
                warn!("Failed to watch {:?}: {e}", wp);
            }
        }

        info!("Watcher running. Ready for script edits.");
        while let Ok(changed_path) = rx.recv() {
            info!("Detected script change at {:?}", changed_path);
            if let Ok(content) = std::fs::read_to_string(&changed_path) {
                let engine = RhaiEngine::new();
                if let Err(err) = engine.compile(&content) {
                    warn!(
                        "Syntax error in script {:?}, retaining active AST: {err}",
                        changed_path
                    );
                    continue;
                }

                if let Some(file_name) = changed_path.file_name().and_then(|n| n.to_str()) {
                    let _ = xdg.sync_origin_to_data(file_name, &content);
                }

                info!("Atomic script swap succeeded. Re-rendering frame...");
                if let Err(e) = render_frame(file, output, width, height, css_path, xdg) {
                    warn!("Re-render error: {e}");
                }
            }
        }
    }

    Ok(())
}

/// Renders a single frame of the HTML document to a target PNG image.
///
/// # Errors
/// Returns `AlloyCliError` on parse, script, or graphic rendering failure.
pub fn render_frame(
    file: &str,
    output: &str,
    width: u32,
    height: u32,
    css_path: Option<&str>,
    xdg: &XdgScriptManager,
) -> Result<(), AlloyCliError> {
    let html_content = std::fs::read_to_string(file)?;
    let dom_tree =
        parse_html(&html_content).map_err(|err| AlloyCliError::HtmlParse(err.to_string()))?;
    let dom_arc = Arc::new(Mutex::new(dom_tree));

    let stylesheet = match css_path {
        Some(css_file) => match std::fs::read_to_string(css_file) {
            Ok(css_content) => {
                parse_css(&css_content).map_err(|err| AlloyCliError::CssParse(err.to_string()))?
            }
            Err(err) => {
                warn!("Failed to read CSS file '{css_file}': {err}");
                StyleSheet::default()
            }
        },
        None => {
            let guard = dom_arc
                .lock()
                .map_err(|_| AlloyCliError::ScriptExecution("Lock poisoned".into()))?;
            extract_inline_style(&guard).unwrap_or_default()
        }
    };

    // 1. Rust Layout Engine
    let guard = dom_arc
        .lock()
        .map_err(|_| AlloyCliError::ScriptExecution("Lock poisoned".into()))?;
    let styled_tree = StyleCascade::build_styled_tree(&guard, &stylesheet);
    let mut display_list = LayoutEngine::layout(&guard, &styled_tree, width as f32, height as f32);
    drop(guard);

    // 2. Rhai Script Pipeline Execution
    let engine = RhaiEngine::new();
    let mut context = engine.create_context(CapabilitySet::all())?;

    let container = ScriptDisplayListContainer::new();
    DomHostModule::new(Arc::clone(&dom_arc)).register(&mut context)?;
    GraphicsHostModule::new(container.clone()).register(&mut context)?;

    let pipeline_src = xdg.resolve_script("pipeline.rhai", DEFAULT_PIPELINE_SCRIPT);
    let _ = engine.eval::<EngineValue>(&mut context, &pipeline_src);

    if let Ok(func_id) = Identifier::new("run_pipeline") {
        if context
            .call_function(
                &func_id,
                &[
                    EngineValue::Float(width as f64),
                    EngineValue::Float(height as f64),
                ],
            )
            .is_ok()
        {
            let script_list = container.get_display_list();
            for cmd in script_list.commands() {
                display_list.push(cmd.clone());
            }
        }
    }

    // 3. Render final DisplayList
    let mut backend = GraphicsBackendFactory::create_headless(width, height);
    backend.render(&display_list)?;
    backend.save_png(Path::new(output))?;

    info!("Rendered '{file}' -> '{output}' ({width}x{height}) successfully.");
    Ok(())
}

/// Extracts CSS declarations from `<style>` tags within the DOM tree.
#[must_use]
pub fn extract_inline_style(dom: &DomTree) -> Option<StyleSheet> {
    let root = dom.root()?;
    let style_tag = TagName::new("style").ok()?;
    let style_nodes = DomService::find_by_tag_name(dom, root, &style_tag);
    let style_node_id = *style_nodes.first()?;
    let css_text = DomService::get_text_content(dom, style_node_id);
    parse_css(&css_text).ok()
}
