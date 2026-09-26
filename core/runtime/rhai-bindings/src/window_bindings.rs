//! Window bindings for Rhai scripts (Fase M, PRD-010, ADR-0003, ADR-0011).
//!
//! Provides the [`WINDOW_BINDINGS`] manifest and registers capability-guarded
//! native functions for window management, repaint scheduling, event routing,
//! and keyboard shortcuts.

use std::collections::BTreeMap;
use std::sync::Arc;

use engine::{
    Arity, Capability, EngineError, EngineValue, ExecutionContext, FunctionName, NativeFn,
    RuntimeEngine, SubsystemName, VariableName, profiles,
};
use rhai_runtime::{PanicHookGuard, RhaiContext, RhaiEngine};
use window::WindowTitle;

/// The manifest of window bindings and their required capabilities.
///
/// Used for capability sweeps (C-06) and fault injection matrices (C-09).
pub const WINDOW_BINDINGS: &[(&str, Capability)] = &[
    ("repaint", Capability::GRAPHICS_DRAW),
    ("title", Capability::WINDOW_MANAGE),
    ("route", Capability::DOM_READ),
    ("key_shortcut", Capability::WINDOW_MANAGE),
];

fn window_error(operation: &str, error_message: impl Into<String>) -> EngineError {
    EngineError::subsystem(SubsystemName::Window, operation, error_message)
}

#[allow(clippy::unnecessary_wraps)]
const fn repaint_handler(arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
    let _ = arguments;
    Ok(EngineValue::Bool(true))
}

fn title_handler(arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
    let title_arg = arguments
        .first()
        .ok_or_else(|| window_error("title", "missing title argument"))?;
    let title_text = match title_arg {
        EngineValue::Text(text) => text.as_str(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    let title = WindowTitle::from(title_text);
    Ok(EngineValue::Text(title.as_str().to_owned()))
}

fn route_handler(arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
    let target_arg = arguments
        .first()
        .ok_or_else(|| window_error("route", "missing target argument"))?;
    let event_arg = arguments
        .get(1)
        .ok_or_else(|| window_error("route", "missing event argument"))?;
    let target_text = match target_arg {
        EngineValue::Text(text) => text.as_str(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    let event_text = match event_arg {
        EngineValue::Text(text) => text.as_str(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    if target_text.is_empty() {
        return Err(window_error("route", "empty route target"));
    }
    let mut route_map = BTreeMap::new();
    route_map.insert(
        "target".to_owned(),
        EngineValue::Text(target_text.to_owned()),
    );
    route_map.insert("event".to_owned(), EngineValue::Text(event_text.to_owned()));
    Ok(EngineValue::Map(route_map))
}

fn key_shortcut_handler(arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
    let key_arg = arguments
        .first()
        .ok_or_else(|| window_error("key_shortcut", "missing key argument"))?;
    let action_arg = arguments
        .get(1)
        .ok_or_else(|| window_error("key_shortcut", "missing action argument"))?;
    let key_text = match key_arg {
        EngineValue::Text(text) => text.as_str(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    let action_text = match action_arg {
        EngineValue::Text(text) => text.as_str(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    if key_text.is_empty() {
        return Err(window_error("key_shortcut", "empty shortcut key"));
    }
    let mut shortcut_map = BTreeMap::new();
    shortcut_map.insert("key".to_owned(), EngineValue::Text(key_text.to_owned()));
    shortcut_map.insert(
        "action".to_owned(),
        EngineValue::Text(action_text.to_owned()),
    );
    Ok(EngineValue::Map(shortcut_map))
}

/// Register window bindings on a Rhai context under capability guards.
pub fn register_window_bindings(context: &mut RhaiContext) -> Result<(), EngineError> {
    let repaint_name = FunctionName::parse("repaint")?;
    let title_name = FunctionName::parse("title")?;
    let route_name = FunctionName::parse("route")?;
    let key_shortcut_name = FunctionName::parse("key_shortcut")?;

    let repaint_fn: NativeFn = Arc::new(repaint_handler);
    let title_fn: NativeFn = Arc::new(title_handler);
    let route_fn: NativeFn = Arc::new(route_handler);
    let key_shortcut_fn: NativeFn = Arc::new(key_shortcut_handler);

    context.register_guarded_binding(
        &repaint_name,
        Arity::exact(0),
        Capability::GRAPHICS_DRAW,
        repaint_fn,
    )?;
    context.register_guarded_binding(
        &title_name,
        Arity::exact(1),
        Capability::WINDOW_MANAGE,
        title_fn,
    )?;
    context.register_guarded_binding(
        &route_name,
        Arity::exact(2),
        Capability::DOM_READ,
        route_fn,
    )?;
    context.register_guarded_binding(
        &key_shortcut_name,
        Arity::exact(2),
        Capability::WINDOW_MANAGE,
        key_shortcut_fn,
    )?;

    Ok(())
}

/// Run a UI script lifecycle hook with fallback safety (C-09).
///
/// Runs under [`profiles::ui_window`]. If the primary script fails, errors,
/// or panics, runs the embedded fallback script, and if that fails, returns a default.
pub fn run_ui_event_with_fallback(
    engine: &RhaiEngine,
    primary_source: &str,
    event_name: &str,
    fallback_source: &str,
) -> EngineValue {
    let primary_result = execute_ui_event(engine, primary_source, event_name);
    if let Ok(value) = primary_result {
        return value;
    }
    tracing::warn!("primary UI script failed, falling back to embedded default");
    let fallback_result = execute_ui_event(engine, fallback_source, event_name);
    if let Ok(value) = fallback_result {
        return value;
    }
    tracing::warn!("fallback UI script failed, returning safe default");
    EngineValue::Bool(true)
}

fn execute_ui_event(
    engine: &RhaiEngine,
    source: &str,
    event_name: &str,
) -> Result<EngineValue, EngineError> {
    let mut context = engine.create_context(profiles::ui_window())?;
    register_window_bindings(&mut context)?;
    let event_var = VariableName::parse("event")?;
    context.set_variable(&event_var, EngineValue::Text(event_name.to_owned()))?;

    let _quiet = PanicHookGuard::install();
    engine.eval_value(&mut context, source)
}
