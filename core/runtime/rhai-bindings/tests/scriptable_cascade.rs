//! Tests for CSS cascade bindings and scriptable resolver (Fase M, PRD-007 §3.4, C-06, C-09).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::significant_drop_tightening
)]

use css::{CascadeResolver, CssColor, StyleSheetSet};
use dom::{DomTree, TagName};
use engine::{Capability, CapabilitySet, EngineError, RuntimeEngine, profiles};
use rhai_bindings::{
    DEFAULT_CASCADE_SCRIPT, ScriptCascadeResolver, SnapshotHandle, StyledTreeHandle,
    register_css_bindings,
};
use rhai_runtime::RhaiEngine;

fn sample_dom() -> DomTree {
    let mut tree = DomTree::new();
    let doc = tree.document();
    let html = tree.create_element(TagName::new("html").expect("html tag"));
    let body = tree.create_element(TagName::new("body").expect("body tag"));
    let h1 = tree.create_element(TagName::new("h1").expect("h1 tag"));

    tree.append_child(doc, html).expect("append html");
    tree.append_child(html, body).expect("append body");
    tree.append_child(body, h1).expect("append h1");
    tree
}

#[test]
fn script_cascade_resolver_alters_computed_style() {
    let engine = RhaiEngine::new();
    let resolver = ScriptCascadeResolver::new(engine, DEFAULT_CASCADE_SCRIPT);

    let dom = sample_dom();
    let snapshot = css::snapshot(&dom, dom.document());
    let sheets = StyleSheetSet::new();

    let styled_tree = resolver.resolve(&snapshot, &sheets).expect("resolve");

    // Find the h1 node in the snapshot
    let h1_id = snapshot
        .nodes_in_document_order()
        .find(|id| {
            snapshot
                .node(*id)
                .and_then(css::NodeRef::tag)
                .is_some_and(|tag| tag == "h1")
        })
        .expect("h1 node found");

    let h1_styled = styled_tree.node(h1_id).expect("h1 styled node");
    assert_eq!(
        h1_styled.style().color(),
        CssColor::rgb(255, 0, 0),
        "cascade.rhai should override h1 color to red"
    );
}

#[test]
fn cascade_bindings_strictly_require_dom_read_and_graphics_draw_never_dom_mutate() {
    let engine = RhaiEngine::new();
    let cascade_caps = profiles::css_cascade();

    // Verify profile contains DOM_READ and GRAPHICS_DRAW, but NOT DOM_MUTATE
    assert!(cascade_caps.contains(Capability::DOM_READ));
    assert!(cascade_caps.contains(Capability::GRAPHICS_DRAW));
    assert!(
        !cascade_caps.contains(Capability::DOM_MUTATE),
        "css_cascade profile must NEVER contain DOM_MUTATE"
    );

    let dom = sample_dom();
    let snapshot = css::snapshot(&dom, dom.document());
    let base_tree = css::UaCascade::new()
        .resolve(&snapshot, &StyleSheetSet::new())
        .expect("base");

    // Try reading with empty capabilities
    let empty_caps = CapabilitySet::empty();
    let mut empty_context = engine.create_context(empty_caps).expect("context");
    register_css_bindings(&mut empty_context).expect("register_css");

    let snapshot_handle = SnapshotHandle::new(std::sync::Arc::new(snapshot), empty_caps);
    let styled_handle = StyledTreeHandle::new(std::sync::Arc::new(base_tree), empty_caps);

    empty_context.set_custom_value(
        &engine::VariableName::parse("dom").expect("dom var"),
        snapshot_handle,
    );
    empty_context.set_custom_value(
        &engine::VariableName::parse("tree").expect("tree var"),
        styled_handle,
    );

    // Reading DOM requires DOM_READ -> PermissionDenied
    let read_outcome = engine.eval_value(&mut empty_context, "dom.len()");
    match read_outcome {
        Err(EngineError::PermissionDenied { capability }) => {
            assert_eq!(capability, Capability::DOM_READ);
        }
        other => panic!("expected PermissionDenied(DOM_READ), got {other:?}"),
    }

    // Setting style requires GRAPHICS_DRAW -> PermissionDenied
    let write_outcome = engine.eval_value(&mut empty_context, r#"tree.set_color(0, "red")"#);
    match write_outcome {
        Err(EngineError::PermissionDenied { capability }) => {
            assert_eq!(capability, Capability::GRAPHICS_DRAW);
        }
        other => panic!("expected PermissionDenied(GRAPHICS_DRAW), got {other:?}"),
    }
}

#[test]
fn panicking_cascade_resolver_recovers_to_ua_default() {
    let engine = RhaiEngine::new();
    let panicking_script = r#"
        let total = dom.len();
        panic("exploding cascade script");
    "#;
    let resolver = ScriptCascadeResolver::new(engine, panicking_script);

    let dom = sample_dom();
    let snapshot = css::snapshot(&dom, dom.document());
    let sheets = StyleSheetSet::new();

    let styled_tree = resolver.resolve(&snapshot, &sheets).expect("resolve");
    let root = styled_tree.node(styled_tree.root()).expect("root node");
    // Baseline root style is valid and preserved
    assert_eq!(root.style().display(), css::Display::Block);
}
