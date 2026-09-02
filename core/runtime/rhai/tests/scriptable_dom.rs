//! **C-03** (`PRD-002:89`, roadmap I1): a Rhai script builds and mutates a
//! host-owned `DomTree` through the global `document` handle, and the host reads
//! the result back. Plus the I1 slice of **C-06 / C-07**: every `NodeHandle`
//! binding is capability-guarded, and a denied capability is
//! `EngineError::PermissionDenied`. (The full panic-injection / conformance
//! sweep is F6.)

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::significant_drop_tightening
)]

use std::sync::{Arc, Mutex};

use dom::{DomTree, serialize_html};
use engine::{Capability, CapabilitySet, EngineError, RuntimeEngine, profiles};
use rhai_runtime::{NODE_HANDLE_BINDINGS, RhaiEngine};

const BUILD_SCRIPT: &str = r#"
    let html = document.create_element("html");
    document.append_child(html);
    let body = html.create_element("body");
    html.append_child(body);
    let para = body.create_element("p");
    body.append_child(para);
    let text = para.create_text("Hello from Rhai");
    para.append_child(text);
    para.set_attribute("class", "greeting");
"#;

fn bound_context(
    engine: &RhaiEngine,
    capabilities: CapabilitySet,
) -> (Arc<Mutex<DomTree>>, <RhaiEngine as RuntimeEngine>::Context) {
    let tree = Arc::new(Mutex::new(DomTree::new()));
    let mut context = engine.create_context(capabilities).expect("context");
    context.bind_dom(Arc::clone(&tree)).expect("bind_dom");
    (tree, context)
}

#[test]
fn a_script_builds_a_dom_tree_and_the_host_reads_it_back() {
    let engine = RhaiEngine::new();
    let (tree, mut context) = bound_context(&engine, profiles::dom_parser());

    engine
        .eval_value(&mut context, BUILD_SCRIPT)
        .expect("the build script runs under DOM_READ | DOM_MUTATE");

    let tree = tree.lock().expect("tree lock");
    let rendered = serialize_html(&tree, tree.document()).expect("serialize");
    assert_eq!(
        rendered,
        r#"<html><body><p class="greeting">Hello from Rhai</p></body></html>"#
    );
}

#[test]
fn a_read_only_context_can_read_but_not_mutate_the_dom() {
    let engine = RhaiEngine::new();
    let (_tree, mut context) = bound_context(&engine, CapabilitySet::new(Capability::DOM_READ));

    engine
        .eval_value(&mut context, "document.children()")
        .expect("a read is allowed under DOM_READ");

    match engine.eval_value(&mut context, r#"document.create_element("div")"#) {
        Err(EngineError::PermissionDenied { capability }) => {
            assert_eq!(capability, Capability::DOM_MUTATE);
        }
        other => panic!("expected PermissionDenied(DOM_MUTATE), got {other:?}"),
    }
}

#[test]
fn every_node_handle_binding_is_capability_guarded() {
    let engine = RhaiEngine::new();
    let (_tree, mut context) = bound_context(&engine, CapabilitySet::empty());

    let snippets = [
        ("tag", "document.tag()"),
        ("text", "document.text()"),
        ("children", "document.children()"),
        ("first_child", "document.first_child()"),
        ("last_child", "document.last_child()"),
        ("previous_sibling", "document.previous_sibling()"),
        ("next_sibling", "document.next_sibling()"),
        ("parent", "document.parent()"),
        ("get_attribute", r#"document.get_attribute("x")"#),
        ("create_element", r#"document.create_element("x")"#),
        ("create_text", r#"document.create_text("x")"#),
        ("append_child", "document.append_child(document)"),
        ("set_text", r#"document.set_text("x")"#),
        ("set_attribute", r#"document.set_attribute("a", "b")"#),
        ("remove_attribute", r#"document.remove_attribute("a")"#),
    ];
    let covered: Vec<&str> = snippets.iter().map(|(name, _)| *name).collect();
    let declared: Vec<&str> = NODE_HANDLE_BINDINGS.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        covered, declared,
        "the guard sweep must exercise exactly the declared binding table"
    );

    for (name, snippet) in snippets {
        let outcome = engine.eval_value(&mut context, snippet);
        assert!(
            matches!(outcome, Err(EngineError::PermissionDenied { .. })),
            "`{name}` must be denied with no capabilities, got {outcome:?}"
        );
    }
}

#[test]
fn script_navigates_dom_via_intrusive_pointer_bindings() {
    let engine = RhaiEngine::new();
    let (_tree, mut context) = bound_context(&engine, profiles::dom_parser());

    let script = r#"
        let html = document.create_element("html");
        document.append_child(html);
        let head = html.create_element("head");
        let body = html.create_element("body");
        html.append_child(head);
        html.append_child(body);

        let first = html.first_child();
        let last = html.last_child();
        let next = first.next_sibling();
        let prev = last.previous_sibling();
        let parent = body.parent();

        first.tag() == "head" &&
        last.tag() == "body" &&
        next.tag() == "body" &&
        prev.tag() == "head" &&
        parent.tag() == "html"
    "#;

    let result: bool = engine.eval(&mut context, script).expect("eval navigation");
    assert!(result);
}

#[test]
fn a_dom_invariant_violation_from_script_maps_to_engine_error_dom() {
    let engine = RhaiEngine::new();
    let (_tree, mut context) = bound_context(&engine, profiles::dom_parser());

    match engine.eval_value(&mut context, r#"document.create_element("1bad")"#) {
        Err(EngineError::Dom { operation, reason }) => {
            assert_eq!(operation, "create_element");
            assert!(reason.contains("tag name"), "reason was: {reason}");
        }
        other => panic!("expected EngineError::Dom, got {other:?}"),
    }
}
