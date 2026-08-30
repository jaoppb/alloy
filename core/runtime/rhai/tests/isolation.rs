//! **C-08**: two `ExecutionContext`s from one `RhaiEngine` share no
//! script-visible state, honour their own capability sets, and a fault in one
//! does not disturb the next evaluation of another.

use std::sync::{Arc, Mutex};

use dom::{DomTree, serialize_html};
use engine::{
    Arity, Capability, CapabilitySet, EngineError, EngineValue, ExecutionContext, RuntimeEngine,
    native_fn, profiles,
};
use rhai_runtime::RhaiEngine;

#[test]
fn contexts_from_one_engine_share_no_scope() {
    let engine = RhaiEngine::new();
    let mut first = engine
        .create_context(CapabilitySet::empty())
        .expect("first");
    let mut second = engine
        .create_context(CapabilitySet::empty())
        .expect("second");

    first.set_variable("secret", 1_i64).expect("set in first");
    second.set_variable("secret", 2_i64).expect("set in second");

    let from_first: i64 = engine.eval(&mut first, "secret").expect("read first");
    let from_second: i64 = engine.eval(&mut second, "secret").expect("read second");
    assert_eq!((from_first, from_second), (1, 2));

    let mut third = engine
        .create_context(CapabilitySet::empty())
        .expect("third");
    assert!(
        engine.eval::<i64>(&mut third, "secret").is_err(),
        "a fresh context sees none of the earlier contexts' variables"
    );
}

#[test]
fn a_guarded_binding_follows_each_context_capability_set() {
    let engine = RhaiEngine::new();
    let handler = native_fn(|_arguments: &[EngineValue]| Ok(EngineValue::Int(7)));

    let mut granted = engine
        .create_context(CapabilitySet::new(Capability::DOM_MUTATE))
        .expect("granted context");
    granted
        .register_guarded_binding(
            "touch",
            Arity::exact(0),
            Capability::DOM_MUTATE,
            handler.clone(),
        )
        .expect("register on granted");
    let allowed: i64 = engine.eval(&mut granted, "touch()").expect("allowed call");
    assert_eq!(allowed, 7);

    let mut denied = engine
        .create_context(CapabilitySet::empty())
        .expect("denied context");
    denied
        .register_guarded_binding("touch", Arity::exact(0), Capability::DOM_MUTATE, handler)
        .expect("register on denied");
    match engine.eval_value(&mut denied, "touch()") {
        Err(EngineError::PermissionDenied { capability }) => {
            assert_eq!(capability, Capability::DOM_MUTATE);
        }
        other => panic!("expected PermissionDenied on the denied context, got {other:?}"),
    }
}

#[test]
fn a_trapped_panic_in_one_context_does_not_disturb_another() {
    let engine = RhaiEngine::new();
    let mut faulting = engine
        .create_context(CapabilitySet::empty())
        .expect("faulting");
    let mut healthy = engine
        .create_context(CapabilitySet::empty())
        .expect("healthy");
    healthy.set_variable("kept", 99_i64).expect("set kept");

    faulting
        .register_fn("boom", || -> i64 {
            panic!("kaboom in the faulting context")
        })
        .expect("register boom");
    let outcome = engine.eval_value(&mut faulting, "boom()");
    assert!(matches!(outcome, Err(EngineError::ScriptPanic { .. })));

    let still_there: i64 = engine.eval(&mut healthy, "kept").expect("read kept");
    assert_eq!(
        still_there, 99,
        "the healthy context is untouched by the trapped panic"
    );
}

#[test]
fn each_bound_context_mutates_only_its_own_dom_tree() {
    let engine = RhaiEngine::new();
    let tree_a = Arc::new(Mutex::new(DomTree::new()));
    let tree_b = Arc::new(Mutex::new(DomTree::new()));
    let mut context_a = engine
        .create_context(profiles::dom_parser())
        .expect("context a");
    let mut context_b = engine
        .create_context(profiles::dom_parser())
        .expect("context b");
    context_a.bind_dom(Arc::clone(&tree_a)).expect("bind a");
    context_b.bind_dom(Arc::clone(&tree_b)).expect("bind b");

    engine
        .eval_value(
            &mut context_a,
            r#"let node = document.create_element("aside"); document.append_child(node);"#,
        )
        .expect("A mutates its own tree");

    let guard_a = tree_a.lock().expect("lock a");
    assert_eq!(
        serialize_html(&guard_a, guard_a.document()).expect("serialize a"),
        "<aside></aside>",
        "A's script landed in A's tree"
    );
    drop(guard_a);

    let guard_b = tree_b.lock().expect("lock b");
    assert_eq!(
        serialize_html(&guard_b, guard_b.document()).expect("serialize b"),
        "",
        "B's tree is untouched by A's script"
    );
}
