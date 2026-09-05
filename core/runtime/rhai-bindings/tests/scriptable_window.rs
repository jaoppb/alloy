//! Tests for window bindings and UI policy (Fase M, PRD-010, C-06, C-07).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::significant_drop_tightening
)]

use engine::{Capability, CapabilitySet, EngineError, EngineValue, RuntimeEngine, profiles};
use rhai_bindings::{
    DEFAULT_UI_SCRIPT, WINDOW_BINDINGS, register_window_bindings, run_ui_event_with_fallback,
};
use rhai_runtime::RhaiEngine;

#[test]
fn each_window_binding_is_denied_without_declared_capability() {
    let engine = RhaiEngine::new();
    let every_capability = Capability::all();

    let snippets = [
        ("repaint", "repaint()"),
        ("title", r#"title("Alloy Window")"#),
        ("route", r#"route("window", "focus")"#),
        ("key_shortcut", r#"key_shortcut("ctrl+w", "close")"#),
    ];

    let declared: Vec<&str> = WINDOW_BINDINGS.iter().map(|(name, _)| *name).collect();
    let covered: Vec<&str> = snippets.iter().map(|(name, _)| *name).collect();
    assert_eq!(declared, covered);

    for (name, required) in WINDOW_BINDINGS {
        let granted = CapabilitySet::new(every_capability.difference(*required));
        let mut context = engine.create_context(granted).expect("context");
        register_window_bindings(&mut context).expect("register_window_bindings");

        let (_, snippet) = snippets.iter().find(|(n, _)| n == name).expect("snippet");
        let outcome = engine.eval_value(&mut context, snippet);
        match outcome {
            Err(EngineError::PermissionDenied { capability }) => {
                assert_eq!(
                    capability, *required,
                    "{name}: expected denial of {required:?}, got {capability:?}"
                );
            }
            other => panic!("{name}: expected PermissionDenied({required:?}), got {other:?}"),
        }
    }
}

#[test]
fn window_bindings_execute_under_ui_window_profile() {
    let engine = RhaiEngine::new();
    let caps = profiles::ui_window();
    let mut context = engine.create_context(caps).expect("context");
    register_window_bindings(&mut context).expect("register_window_bindings");

    let repaint_res = engine
        .eval_value(&mut context, "repaint()")
        .expect("repaint");
    assert_eq!(repaint_res, EngineValue::Bool(true));

    let title_res = engine
        .eval_value(&mut context, r#"title("Test Title")"#)
        .expect("title");
    assert_eq!(title_res, EngineValue::Text("Test Title".to_owned()));

    let route_res = engine
        .eval_value(&mut context, r#"route("main", "click")"#)
        .expect("route");
    assert!(matches!(route_res, EngineValue::Map(_)));

    let shortcut_res = engine
        .eval_value(&mut context, r#"key_shortcut("ctrl+q", "quit")"#)
        .expect("shortcut");
    assert!(matches!(shortcut_res, EngineValue::Map(_)));
}

#[test]
fn default_ui_script_handles_events() {
    let engine = RhaiEngine::new();
    let outcome =
        run_ui_event_with_fallback(&engine, DEFAULT_UI_SCRIPT, "resize", DEFAULT_UI_SCRIPT);
    assert_eq!(outcome, EngineValue::Bool(true));
}
