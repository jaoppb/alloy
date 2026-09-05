//! Window bindings for Rhai scripts (Fase M, PRD-010, ADR-0003, ADR-0010, ADR-0011).
//!
//! Provides the [`WINDOW_BINDINGS`] manifest and registers capability-guarded
//! native functions for window management, repaint scheduling, event routing,
//! and keyboard shortcuts.

use std::collections::BTreeMap;
use std::sync::Arc;

use engine::{
    Arity, Capability, EngineError, EngineValue, ExecutionContext, NativeFn, RuntimeEngine,
    SubsystemName, VariableName, profiles,
};
use rhai_runtime::{
    GuardedBinding, PanicHookGuard, RhaiContext, RhaiEngine, install_guarded_table,
    run_with_fallback,
};
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
const fn repaint_handler(_arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
    Ok(EngineValue::Bool(true))
}

fn title_handler(arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
    let title_argument = arguments
        .first()
        .ok_or_else(|| window_error("title", "missing title argument"))?;
    let title_text = match title_argument {
        EngineValue::Text(text) => text.as_str(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    let title = WindowTitle::from(title_text);
    let title_string = title.as_str().to_owned();
    Ok(EngineValue::Text(title_string))
}

fn route_handler(arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
    let target_argument = arguments
        .first()
        .ok_or_else(|| window_error("route", "missing target argument"))?;
    let event_argument = arguments
        .get(1)
        .ok_or_else(|| window_error("route", "missing event argument"))?;
    let target_text = match target_argument {
        EngineValue::Text(text) => text.as_str(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    let event_text = match event_argument {
        EngineValue::Text(text) => text.as_str(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    if target_text.is_empty() {
        return Err(window_error("route", "empty route target"));
    }
    let mut route_map = BTreeMap::new();
    let target_value = EngineValue::Text(target_text.to_owned());
    route_map.insert("target".to_owned(), target_value);
    let event_value = EngineValue::Text(event_text.to_owned());
    route_map.insert("event".to_owned(), event_value);
    Ok(EngineValue::Map(route_map))
}

fn key_shortcut_handler(arguments: &[EngineValue]) -> Result<EngineValue, EngineError> {
    let key_argument = arguments
        .first()
        .ok_or_else(|| window_error("key_shortcut", "missing key argument"))?;
    let action_argument = arguments
        .get(1)
        .ok_or_else(|| window_error("key_shortcut", "missing action argument"))?;
    let key_text = match key_argument {
        EngineValue::Text(text) => text.as_str(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    let action_text = match action_argument {
        EngineValue::Text(text) => text.as_str(),
        other => {
            return Err(EngineError::type_mismatch("Text", other.kind().name()));
        }
    };
    if key_text.is_empty() {
        return Err(window_error("key_shortcut", "empty shortcut key"));
    }
    let mut shortcut_map = BTreeMap::new();
    let key_value = EngineValue::Text(key_text.to_owned());
    shortcut_map.insert("key".to_owned(), key_value);
    let action_value = EngineValue::Text(action_text.to_owned());
    shortcut_map.insert("action".to_owned(), action_value);
    Ok(EngineValue::Map(shortcut_map))
}

/// Builds the table of guarded window bindings for [`install_guarded_table`].
#[must_use]
pub fn window_guarded_bindings() -> [GuardedBinding; 4] {
    let repaint_handler_fn: NativeFn = Arc::new(repaint_handler);
    let title_handler_fn: NativeFn = Arc::new(title_handler);
    let route_handler_fn: NativeFn = Arc::new(route_handler);
    let key_shortcut_handler_fn: NativeFn = Arc::new(key_shortcut_handler);

    [
        GuardedBinding::new(
            "repaint",
            Arity::exact(0),
            Capability::GRAPHICS_DRAW,
            repaint_handler_fn,
        ),
        GuardedBinding::new(
            "title",
            Arity::exact(1),
            Capability::WINDOW_MANAGE,
            title_handler_fn,
        ),
        GuardedBinding::new(
            "route",
            Arity::exact(2),
            Capability::DOM_READ,
            route_handler_fn,
        ),
        GuardedBinding::new(
            "key_shortcut",
            Arity::exact(2),
            Capability::WINDOW_MANAGE,
            key_shortcut_handler_fn,
        ),
    ]
}

/// Register window bindings on a Rhai context under capability guards.
pub fn register_window_bindings(context: &mut RhaiContext) -> Result<(), EngineError> {
    let bindings = window_guarded_bindings();
    install_guarded_table(context, &bindings)
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
    let (outcome, returned) = run_with_fallback(
        None,
        || execute_ui_event(engine, primary_source, event_name).map(|val| (val.clone(), val)),
        || execute_ui_event(engine, fallback_source, event_name),
        || EngineValue::Bool(true),
    );
    returned.unwrap_or(outcome)
}

fn execute_ui_event(
    engine: &RhaiEngine,
    source: &str,
    event_name: &str,
) -> Result<EngineValue, EngineError> {
    let mut context = engine.create_context(profiles::ui_window())?;
    register_window_bindings(&mut context)?;
    let event_variable = VariableName::parse("event")?;
    let event_value = EngineValue::Text(event_name.to_owned());
    context.set_variable(&event_variable, event_value)?;

    let _quiet = PanicHookGuard::install();
    engine.eval_value(&mut context, source)
}
