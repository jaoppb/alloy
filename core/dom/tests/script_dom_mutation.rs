use dom::{DomService, DomTree, register_dom_bindings};
use engine::{
    Capability, CapabilitySet, EngineError, EngineValue, ExecutionContext, Identifier,
    RuntimeEngine,
};
use rhai_runtime::RhaiEngine;
use std::sync::{Arc, Mutex};

#[test]
fn test_c03_dom_node_readable_and_mutable_from_script() {
    let tree = Arc::new(Mutex::new(DomTree::new()));
    let engine = RhaiEngine::new();

    // Context with DOM read and mutate privileges
    let mut context = engine
        .create_context(CapabilitySet::new(
            Capability::DOM_READ | Capability::DOM_MUTATE,
        ))
        .expect("Context creation should succeed");

    // Register DOM domain bindings into the execution isolate
    register_dom_bindings(&mut context, Arc::clone(&tree))
        .expect("DOM bindings registration must succeed");

    // 1. Script creates an element node 'h1'
    let h1_id = context
        .call_function(
            &Identifier::new("dom_create_element").unwrap(),
            &[EngineValue::String("h1".to_string())],
        )
        .expect("Should create element");

    // 2. Script creates a text node 'Initial Content'
    let text_id = context
        .call_function(
            &Identifier::new("dom_create_text").unwrap(),
            &[EngineValue::String("Initial Content".to_string())],
        )
        .expect("Should create text node");

    // 3. Script appends text node under 'h1'
    context
        .call_function(
            &Identifier::new("dom_append_child").unwrap(),
            &[h1_id.clone(), text_id.clone()],
        )
        .expect("Should append child");

    // 4. Script reads the text content
    let read_val = context
        .call_function(
            &Identifier::new("dom_get_text").unwrap(),
            std::slice::from_ref(&text_id),
        )
        .expect("Should read text");
    assert_eq!(read_val.as_str().unwrap(), "Initial Content");

    // 5. Script mutates the text node to 'Updated by Script'
    context
        .call_function(
            &Identifier::new("dom_set_text").unwrap(),
            &[
                text_id.clone(),
                EngineValue::String("Updated by Script".to_string()),
            ],
        )
        .expect("Should mutate text");

    // 6. Verify the mutation inside the Rust DomTree aggregate
    {
        let guard = tree.lock().unwrap();
        let h1_node_id = dom::NodeId::new(h1_id.as_i64().unwrap() as u32);
        let serialized = DomService::serialize_to_html(&guard, h1_node_id);
        assert_eq!(serialized, "<h1>Updated by Script</h1>");
    }
}

#[test]
fn test_dom_mutation_denied_without_capability() {
    let tree = Arc::new(Mutex::new(DomTree::new()));
    let engine = RhaiEngine::new();

    // Context with DOM_READ only (DOM_MUTATE omitted)
    let mut context = engine
        .create_context(CapabilitySet::new(Capability::DOM_READ))
        .expect("Context creation should succeed");

    register_dom_bindings(&mut context, Arc::clone(&tree))
        .expect("DOM bindings registration must succeed");

    // Attempting to create an element without DOM_MUTATE must fail with PermissionDenied
    let result = context.call_function(
        &Identifier::new("dom_create_element").unwrap(),
        &[EngineValue::String("div".to_string())],
    );

    match result {
        Err(EngineError::PermissionDenied(cap)) => {
            assert!(cap.contains("DOM_MUTATE"));
        }
        other => panic!("Expected EngineError::PermissionDenied, got: {other:?}"),
    }
}
