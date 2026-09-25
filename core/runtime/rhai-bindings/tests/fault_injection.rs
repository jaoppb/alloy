//! **C-09** — the panic-injection gate (roadmap §5: blocking from v0.2).
//!
//! For every capability, a guarded binding whose handler `panic!`s is trapped
//! as `EngineError::ScriptPanic`, the test process stays alive, and the context
//! remains usable. `run_dom_with_fallback` recovers from every failure class,
//! including the embedded default script itself failing.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::similar_names,
    clippy::significant_drop_tightening
)]

use css::{CascadeResolver, StyleSheetSet};
use engine::{
    Arity, Capability, CapabilitySet, EngineError, EngineValue, ExecutionLimits, FunctionName,
    RuntimeEngine, native_fn, profiles,
};
use network::{HttpRequest, PolicyVerdict, RequestPolicy, Url};
use rhai_bindings::{
    NETWORK_BINDINGS, ScriptCascadeResolver, ScriptRequestPolicy, WINDOW_BINDINGS,
    minimal_document, run_dom_with_fallback, run_ui_event_with_fallback,
};
use rhai_runtime::RhaiEngine;

const ALL_CAPABILITIES: [Capability; 9] = [
    Capability::DOM_READ,
    Capability::DOM_MUTATE,
    Capability::NETWORK_FETCH,
    Capability::NETWORK_LISTEN,
    Capability::FS_READ_SCRIPTS,
    Capability::FS_WRITE_CACHE,
    Capability::GRAPHICS_DRAW,
    Capability::WINDOW_MANAGE,
    Capability::DEVTOOLS_INSPECT,
];

const DEFAULT_DOM: &str = "\
    let html = document.create_element(\"html\"); \
    document.append_child(html); \
    let body = html.create_element(\"body\"); \
    html.append_child(body);";

#[test]
fn a_panicking_guarded_binding_is_trapped_for_every_capability() {
    let engine = RhaiEngine::new();
    let explode = FunctionName::parse("explode").expect("valid function name");
    for capability in ALL_CAPABILITIES {
        let mut context = engine
            .create_context(CapabilitySet::new(capability))
            .expect("context");
        let handler = native_fn(
            |_arguments: &[EngineValue]| -> Result<EngineValue, EngineError> {
                panic!("injected panic in a guarded binding")
            },
        );
        context
            .register_guarded_binding(&explode, Arity::exact(0), capability, handler)
            .expect("register explode");

        let outcome = engine.eval_value(&mut context, "explode()");
        assert!(
            matches!(outcome, Err(EngineError::ScriptPanic { .. })),
            "capability {capability:?}: expected a trapped ScriptPanic, got {outcome:?}"
        );

        let after: i64 = engine
            .eval(&mut context, "1 + 1")
            .expect("the context is still usable after a trapped panic");
        assert_eq!(
            after, 2,
            "capability {capability:?}: context broke after a trapped panic"
        );
    }
}

#[test]
fn run_dom_with_fallback_contains_every_failure_mode() {
    let engine = RhaiEngine::new();

    // 1. Each broken-primary class (compile / DOM / runtime) falls back to the
    //    embedded default document.
    let broken_primaries = [
        ("syntax error", "@@@ not rhai @@@"),
        (
            "DOM invariant violation",
            r#"document.create_element("1bad")"#,
        ),
        ("runtime error", "let items = [1]; items[9]"),
    ];
    for (label, primary) in broken_primaries {
        let (tree, value) =
            run_dom_with_fallback(&engine, profiles::dom_parser(), primary, None, DEFAULT_DOM);
        assert!(
            value.is_none(),
            "{label}: a fallback path carries no primary value"
        );
        assert_eq!(
            dom::serialize_html(&tree, tree.document()).expect("serialize"),
            "<html><body></body></html>",
            "{label}: fell back to the embedded default document"
        );
    }

    // The execution-limit class, on a deliberately tiny ceiling so the test is fast.
    let tight = RhaiEngine::with_limits(ExecutionLimits::strict().with_max_operations(20_000));
    let (tree, value) = run_dom_with_fallback(
        &tight,
        profiles::dom_parser(),
        "let n = 0; while true { n += 1; }",
        None,
        DEFAULT_DOM,
    );
    assert!(value.is_none(), "execution-limit breach: no primary value");
    assert_eq!(
        dom::serialize_html(&tree, tree.document()).expect("serialize"),
        "<html><body></body></html>",
        "execution-limit breach: fell back to the embedded default document"
    );

    // A capability-denied primary also falls back. Here the caps also deny the
    // default script, so it is `minimal_document()` that wins — still well-formed.
    let (tree, value) = run_dom_with_fallback(
        &engine,
        CapabilitySet::new(Capability::DOM_READ),
        r#"document.create_element("div")"#,
        None,
        DEFAULT_DOM,
    );
    assert!(value.is_none(), "permission denied: no primary value");
    assert_eq!(
        dom::serialize_html(&tree, tree.document()).expect("serialize"),
        "<html><body></body></html>",
        "permission denied: fell back to a well-formed document"
    );

    // 2. When the embedded default *also* fails, the Rust minimal document wins.
    let (tree, value) = run_dom_with_fallback(
        &engine,
        profiles::dom_parser(),
        "@@@ broken primary @@@",
        None,
        "@@@ broken fallback @@@",
    );
    assert!(value.is_none());
    assert_eq!(
        dom::serialize_html(&tree, tree.document()).expect("serialize"),
        "<html><body></body></html>",
        "a broken default script falls through to minimal_document()"
    );

    // 3. A healthy primary keeps its return value and its tree.
    let (tree, value) = run_dom_with_fallback(
        &engine,
        profiles::dom_parser(),
        "let h = document.create_element(\"section\"); document.append_child(h); 41 + 1",
        None,
        DEFAULT_DOM,
    );
    assert_eq!(value, Some(EngineValue::Int(42)));
    assert_eq!(
        dom::serialize_html(&tree, tree.document()).expect("serialize"),
        "<section></section>"
    );
}

#[test]
fn the_fallback_runs_on_a_clean_tree_not_the_primary_partial() {
    let engine = RhaiEngine::new();
    // The primary builds <html> and *then* fails. If the fallback reused that
    // partial tree, the default script would append a second <html>.
    let partial_then_fail = r#"
        let html = document.create_element("html");
        document.append_child(html);
        let bad = [1];
        bad[9]
    "#;
    let (tree, value) = run_dom_with_fallback(
        &engine,
        profiles::dom_parser(),
        partial_then_fail,
        None,
        DEFAULT_DOM,
    );

    assert!(value.is_none());
    assert_eq!(
        dom::serialize_html(&tree, tree.document()).expect("serialize"),
        "<html><body></body></html>",
        "the fallback started from a fresh DomTree, discarding the primary's partial <html>"
    );
}

#[test]
fn minimal_document_is_well_formed() {
    let tree = minimal_document();
    assert_eq!(
        dom::serialize_html(&tree, tree.document()).expect("serialize"),
        "<html><body></body></html>"
    );
}

fn network_snippet(name: &str) -> (&'static str, Arity) {
    match name {
        "fetch" => ("fetch(\"https://example.com\")", Arity::exact(1)),
        "allow" => ("allow(\"request\")", Arity::exact(1)),
        "deny" => ("deny(\"blocked\")", Arity::exact(1)),
        "rewrite" => ("rewrite(\"https://example.com\")", Arity::exact(1)),
        "header" => ("header(\"content-type\", \"text/html\")", Arity::exact(2)),
        other => panic!("unknown network binding `{other}`"),
    }
}

fn window_snippet(name: &str) -> (&'static str, Arity) {
    match name {
        "repaint" => ("repaint()", Arity::exact(0)),
        "title" => ("title(\"test\")", Arity::exact(1)),
        "route" => ("route(\"window\", \"focus\")", Arity::exact(2)),
        "key_shortcut" => ("key_shortcut(\"ctrl+q\", \"quit\")", Arity::exact(2)),
        other => panic!("unknown window binding `{other}`"),
    }
}

#[test]
fn a_panicking_network_binding_is_trapped_as_script_panic() {
    let engine = RhaiEngine::new();
    for (name, capability) in NETWORK_BINDINGS {
        let fn_name = FunctionName::parse(name).expect("valid function name");
        let (snippet, arity) = network_snippet(name);
        let mut context = engine
            .create_context(CapabilitySet::new(*capability))
            .expect("context");

        let handler = native_fn(
            |_arguments: &[EngineValue]| -> Result<EngineValue, EngineError> {
                panic!("injected panic in network binding");
            },
        );
        context
            .register_guarded_binding(&fn_name, arity, *capability, handler)
            .expect("register panicking binding");

        let outcome = engine.eval_value(&mut context, snippet);
        assert!(
            matches!(outcome, Err(EngineError::ScriptPanic { .. })),
            "binding {name}: expected trapped ScriptPanic, got {outcome:?}"
        );

        let after: i64 = engine
            .eval(&mut context, "1 + 1")
            .expect("context still usable after trapped network panic");
        assert_eq!(after, 2);
    }
}

#[test]
fn a_panicking_window_binding_is_trapped_as_script_panic() {
    let engine = RhaiEngine::new();
    for (name, capability) in WINDOW_BINDINGS {
        let fn_name = FunctionName::parse(name).expect("valid function name");
        let (snippet, arity) = window_snippet(name);
        let mut context = engine
            .create_context(CapabilitySet::new(*capability))
            .expect("context");

        let handler = native_fn(
            |_arguments: &[EngineValue]| -> Result<EngineValue, EngineError> {
                panic!("injected panic in window binding");
            },
        );
        context
            .register_guarded_binding(&fn_name, arity, *capability, handler)
            .expect("register panicking binding");

        let outcome = engine.eval_value(&mut context, snippet);
        assert!(
            matches!(outcome, Err(EngineError::ScriptPanic { .. })),
            "binding {name}: expected trapped ScriptPanic, got {outcome:?}"
        );

        let after: i64 = engine
            .eval(&mut context, "1 + 1")
            .expect("context still usable after trapped window panic");
        assert_eq!(after, 2);
    }
}

#[test]
fn panicking_network_script_policy_falls_back_safely() {
    let engine = RhaiEngine::new();
    let panicking_script = "panic(\"injected network script fault\");";
    let policy = ScriptRequestPolicy::new(engine, panicking_script);

    let url = Url::parse("https://example.com").expect("url");
    let request = HttpRequest::get(url);
    let verdict = policy.decide(&request).expect("verdict via fallback");
    assert_eq!(verdict, PolicyVerdict::Allow);
}

#[test]
fn panicking_ui_script_falls_back_safely() {
    let engine = RhaiEngine::new();
    let panicking_script = "panic(\"injected UI script fault\");";
    let fallback_script = "repaint(); 42";

    let outcome = run_ui_event_with_fallback(&engine, panicking_script, "resize", fallback_script);
    assert_eq!(outcome, EngineValue::Int(42));
}

#[test]
fn panicking_cascade_script_falls_back_to_ua_cascade() {
    let engine = RhaiEngine::new();
    let panicking_script = "panic(\"injected cascade fault\");";
    let resolver = ScriptCascadeResolver::new(engine, panicking_script);

    let dom_tree = minimal_document();
    let snapshot = css::snapshot(&dom_tree, dom_tree.document());
    let sheets = StyleSheetSet::new();

    let outcome = resolver.resolve(&snapshot, &sheets);
    assert!(
        outcome.is_ok(),
        "cascade should fall back to UaCascade on panic"
    );
}
