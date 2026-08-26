use engine::{
    Capability, CapabilitySet, EngineError, EngineValue, ExecutionContext, FromEngineValue,
    Identifier, IntoEngineValue, MockEngine, NativeFn, RuntimeEngine,
};
use std::sync::{Arc, Mutex};

/// Domain entity simulated in test to verify C-05:
/// Domain entities can be manipulated via abstract engine traits without coupling to concrete interpreters.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TestDomNode {
    tag: String,
    text: String,
}

impl TestDomNode {
    fn new(tag: &str, text: &str) -> Self {
        Self {
            tag: tag.to_string(),
            text: text.to_string(),
        }
    }
}

// C-01: RuntimeEngine and ExecutionContext traits defined in core/engine
#[test]
fn test_c01_runtime_engine_and_context_traits_defined() {
    let engine = MockEngine::new();
    let capabilities = CapabilitySet::new(Capability::DOM_READ);
    let mut context = engine
        .create_context(capabilities)
        .expect("Context creation should succeed");

    assert!(context.capabilities().contains(Capability::DOM_READ));
    assert!(!context.capabilities().contains(Capability::DOM_MUTATE));

    let script = engine.compile("100").expect("Compilation should succeed");
    assert_eq!(script, "100");

    let result: i64 = engine
        .eval(&mut context, "100")
        .expect("Eval should succeed");
    assert_eq!(result, 100);
}

// C-05: Test with mocked engine proving interchangeability without touching domain structs
#[test]
fn test_c05_engine_mocked_proves_interchangeability_without_coupling_domain() {
    let engine = MockEngine::new();
    let capabilities = CapabilitySet::new(Capability::DOM_READ | Capability::DOM_MUTATE);
    let mut context = engine
        .create_context(capabilities)
        .expect("Context creation must succeed");

    // Shared domain state
    let dom_node = Arc::new(Mutex::new(TestDomNode::new("div", "initial text")));

    // Register a native host mutation function that modifies the domain struct
    let node_clone = Arc::clone(&dom_node);
    let mutate_fn: NativeFn = Arc::new(move |ctx, _args| {
        // Enforce capability gate
        if !ctx.capabilities().contains(Capability::DOM_MUTATE) {
            return Err(EngineError::PermissionDenied(
                "Missing DOM_MUTATE capability".to_string(),
            ));
        }

        let mut node = node_clone.lock().unwrap();
        node.text = "mutated by script".to_string();
        Ok(EngineValue::String(node.text.clone()))
    });

    let fn_id = Identifier::new("mutate_dom").unwrap();
    context
        .register_fn(fn_id, mutate_fn)
        .expect("Function registration must succeed");

    // Execute through engine
    let result: String = engine
        .eval(&mut context, "mutate_dom()")
        .expect("Evaluation must succeed");

    assert_eq!(result, "mutated by script");
    assert_eq!(dom_node.lock().unwrap().text, "mutated by script");
}

#[test]
fn test_capability_sandbox_blocks_unauthorized_action() {
    let engine = MockEngine::new();
    // Only grant DOM_READ, omitting DOM_MUTATE
    let capabilities = CapabilitySet::new(Capability::DOM_READ);
    let mut context = engine.create_context(capabilities).unwrap();

    let mutate_fn: NativeFn = Arc::new(|ctx, _args| {
        if !ctx.capabilities().contains(Capability::DOM_MUTATE) {
            return Err(EngineError::PermissionDenied(
                "Write operation prohibited".to_string(),
            ));
        }
        Ok(EngineValue::Bool(true))
    });

    context
        .register_fn(Identifier::new("mutate").unwrap(), mutate_fn)
        .unwrap();

    let err = engine.eval::<bool>(&mut context, "mutate()").unwrap_err();
    assert_eq!(
        err,
        EngineError::PermissionDenied("Write operation prohibited".to_string())
    );
}

#[test]
fn test_identifier_validation() {
    assert!(Identifier::new("valid_name").is_ok());
    assert!(Identifier::new("  trimmed_name  ").is_ok());
    assert_eq!(
        Identifier::new("  trimmed_name  ").unwrap().as_str(),
        "trimmed_name"
    );

    assert!(Identifier::new("").is_err());
    assert!(Identifier::new("   ").is_err());
}

#[test]
fn test_variable_get_set_reset_scope() {
    let engine = MockEngine::new();
    let mut context = engine.create_context(CapabilitySet::empty()).unwrap();

    let var_id = Identifier::new("counter").unwrap();
    context
        .set_variable(var_id.clone(), 42.into_engine_value())
        .unwrap();

    let val = context.get_variable(&var_id).unwrap().unwrap();
    assert_eq!(val.as_i64().unwrap(), 42);

    let evaluated: i64 = engine.eval(&mut context, "counter").unwrap();
    assert_eq!(evaluated, 42);

    // Reset scope
    context.reset_scope().unwrap();
    assert!(context.get_variable(&var_id).unwrap().is_none());
}

#[test]
fn test_type_conversion_and_type_mismatch() {
    let int_val = EngineValue::Int(42);
    assert_eq!(i64::from_engine_value(&int_val).unwrap(), 42);
    assert_eq!(f64::from_engine_value(&int_val).unwrap(), 42.0);

    let err = bool::from_engine_value(&int_val).unwrap_err();
    assert_eq!(
        err,
        EngineError::TypeMismatch {
            expected: "bool",
            found: "Int",
        }
    );

    let str_val = EngineValue::String("hello".to_string());
    assert_eq!(String::from_engine_value(&str_val).unwrap(), "hello");
}
