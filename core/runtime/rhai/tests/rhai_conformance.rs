use engine::{
    Capability, CapabilitySet, EngineError, EngineValue, ExecutionContext, HostObject, Identifier,
    IntoEngineValue, RuntimeEngine,
};
use rhai_runtime::{ExecutionLimits, RhaiEngine};

#[test]
fn test_c02_rhai_engine_conformance() {
    let engine = RhaiEngine::new();
    let mut context = engine
        .create_context(CapabilitySet::new(Capability::DOM_READ))
        .expect("Context creation should succeed");

    assert!(context.capabilities().contains(Capability::DOM_READ));
    assert!(!context.capabilities().contains(Capability::DOM_MUTATE));

    // Evaluate arithmetic expression
    let result: i64 = engine
        .eval(&mut context, "40 + 2")
        .expect("Evaluation must succeed");
    assert_eq!(result, 42);

    // Evaluate boolean expression
    let flag: bool = engine
        .eval(&mut context, "10 > 5")
        .expect("Boolean eval must succeed");
    assert!(flag);

    // Evaluate string concatenation
    let text: String = engine
        .eval(&mut context, "\"Alloy \" + \"Browser\"")
        .expect("String eval must succeed");
    assert_eq!(text, "Alloy Browser");
}

#[test]
fn test_c04_infinite_loop_aborted_by_execution_limits() {
    // Restrict max operations to 500 steps to trigger immediate abort
    let limits = ExecutionLimits::new().with_max_operations(500);
    let engine = RhaiEngine::new().with_limits(limits);
    let mut context = engine
        .create_context(CapabilitySet::empty())
        .expect("Context creation must succeed");

    // Execute infinite loop
    let result: Result<i64, EngineError> = engine.eval(&mut context, "while true {}");

    match result {
        Err(EngineError::ExecutionLimitExceeded(msg)) => {
            assert!(
                msg.contains("too many operations") || msg.contains("limit exceeded"),
                "Expected operation limit error message, got: {msg}"
            );
        }
        other => panic!("Expected EngineError::ExecutionLimitExceeded, got: {other:?}"),
    }
}

#[test]
fn test_syntax_error_reported_as_engine_error() {
    let engine = RhaiEngine::new();
    let mut context = engine.create_context(CapabilitySet::empty()).unwrap();

    let result: Result<i64, EngineError> = engine.eval(&mut context, "let x = ;");
    match result {
        Err(EngineError::SyntaxError(msg)) => {
            assert!(!msg.is_empty(), "Syntax error must contain description");
        }
        other => panic!("Expected EngineError::SyntaxError, got: {other:?}"),
    }
}

#[test]
fn test_variable_scope_and_reset() {
    let engine = RhaiEngine::new();
    let mut context = engine.create_context(CapabilitySet::empty()).unwrap();

    let var_name = Identifier::new("counter").unwrap();
    context
        .set_variable(var_name.clone(), 10.into_engine_value())
        .unwrap();

    let retrieved = context.get_variable(&var_name).unwrap().unwrap();
    assert_eq!(retrieved.as_i64().unwrap(), 10);

    let evaluated: i64 = engine.eval(&mut context, "counter * 3").unwrap();
    assert_eq!(evaluated, 30);

    // Reset scope
    context.reset_scope().unwrap();
    assert!(context.get_variable(&var_name).unwrap().is_none());
}

#[test]
fn test_compile_and_eval_ast() {
    let engine = RhaiEngine::new();
    let mut context = engine.create_context(CapabilitySet::empty()).unwrap();

    let ast = engine
        .compile("let a = 10; let b = 32; a + b")
        .expect("Compilation should succeed");

    let result: i64 = engine
        .raw_engine()
        .eval_ast_with_scope(context.scope_mut(), &ast)
        .expect("Evaluation of compiled AST must succeed");

    assert_eq!(result, 42);
}

#[test]
fn test_host_object_accessible_via_script_eval() {
    let engine = RhaiEngine::new();
    let mut context = engine
        .create_context(CapabilitySet::new(Capability::DOM_MUTATE))
        .expect("Context creation");

    let mut doc_obj = HostObject::new(Identifier::new("document").unwrap());
    doc_obj.add_method(Identifier::new("createElement").unwrap(), |_this, args| {
        let tag = args
            .first()
            .and_then(|a| a.as_str().ok())
            .unwrap_or("unknown");
        Ok(EngineValue::String(format!("<{tag}></{tag}>")))
    });

    context.register_host_object(doc_obj).unwrap();

    // Call via engine.eval directly (proves D-01 is fixed!)
    let result: String = engine
        .eval(&mut context, r#"document.createElement("div")"#)
        .expect("Eval of host object method must succeed");

    assert_eq!(result, "<div></div>");
}

#[test]
fn test_host_object_permission_denied_via_script_eval() {
    let engine = RhaiEngine::new();
    // Context without DOM_MUTATE
    let mut context = engine
        .create_context(CapabilitySet::empty())
        .expect("Context creation");

    let mut doc_obj = HostObject::new(Identifier::new("document").unwrap())
        .with_capability(Capability::DOM_MUTATE);
    doc_obj.add_method(Identifier::new("createElement").unwrap(), |_this, _args| {
        Ok(EngineValue::String("created".into()))
    });

    context.register_host_object(doc_obj).unwrap();

    let result: Result<String, EngineError> =
        engine.eval(&mut context, r#"document.createElement("div")"#);
    assert!(
        result.is_err(),
        "Expected permission denied error when capability is missing"
    );
}
