use engine::{
    Capability, EngineError, EngineValue, ExecutionContext, Identifier, MockContext,
    SubsystemProfile, TrappedExecutor, guarded_native_fn,
};

#[test]
fn test_c06_c07_guarded_native_function_enforces_capabilities() {
    let fn_id = Identifier::new("network_fetch").unwrap();

    let native_fetch = guarded_native_fn(Capability::NETWORK_FETCH, |_ctx, args| {
        let url = args
            .first()
            .and_then(|v| v.as_str().ok())
            .unwrap_or("http://default");
        Ok(EngineValue::String(format!("fetched: {url}")))
    });

    // 1. Context without NETWORK_FETCH (C-07: unauthorized returns PermissionDenied)
    let mut restricted_ctx = MockContext::new(SubsystemProfile::dom_parser());
    restricted_ctx
        .register_fn(fn_id.clone(), native_fetch.clone())
        .unwrap();

    let denied_result = restricted_ctx.call_function(
        &fn_id,
        &[EngineValue::String("https://alloy.org".to_string())],
    );

    match denied_result {
        Err(EngineError::PermissionDenied(cap)) => {
            assert!(cap.contains("NETWORK_FETCH"));
        }
        other => panic!("Expected EngineError::PermissionDenied, got: {other:?}"),
    }

    // 2. Context with NETWORK_FETCH (C-06: authorized executes successfully)
    let mut authorized_ctx = MockContext::new(SubsystemProfile::network_interceptor());
    authorized_ctx
        .register_fn(fn_id.clone(), native_fetch)
        .unwrap();

    let success_result = authorized_ctx
        .call_function(
            &fn_id,
            &[EngineValue::String("https://alloy.org".to_string())],
        )
        .expect("Authorized context should execute");

    assert_eq!(
        success_result.as_str().unwrap(),
        "fetched: https://alloy.org"
    );
}

#[test]
fn test_c08_subsystems_maintain_isolated_contexts_and_scopes() {
    let mut dom_ctx = MockContext::new(SubsystemProfile::dom_parser());
    let mut css_ctx = MockContext::new(SubsystemProfile::css_cascade());

    let var_id = Identifier::new("shared_identifier").unwrap();

    // 1. Set variable in DOM context
    dom_ctx
        .set_variable(var_id.clone(), EngineValue::Int(100))
        .unwrap();

    // 2. Verify CSS context has no access to DOM context's variable
    let css_lookup = css_ctx.get_variable(&var_id).unwrap();
    assert_eq!(
        css_lookup, None,
        "CSS context must not observe DOM variables"
    );

    // 3. Set same identifier with different value in CSS context
    css_ctx
        .set_variable(var_id.clone(), EngineValue::String("css-rule".to_string()))
        .unwrap();

    // 4. Verify neither context pollutes the other
    let dom_val = dom_ctx.get_variable(&var_id).unwrap().unwrap();
    let css_val = css_ctx.get_variable(&var_id).unwrap().unwrap();

    assert_eq!(dom_val.as_i64().unwrap(), 100);
    assert_eq!(css_val.as_str().unwrap(), "css-rule");
}

#[test]
fn test_c09_panicking_script_trapped_without_crashing_host_and_invokes_fallback() {
    // Action simulating a closure or script panicking during execution
    let panicking_action = || -> Result<String, EngineError> {
        panic!("Fatal internal script panic: index out of bounds");
    };

    // Trapped execution: host stays alive, fallback is invoked
    let result = TrappedExecutor::execute_with_fallback(panicking_action, |trapped_err| {
        match trapped_err {
            EngineError::PanicTrapped(msg) => {
                assert!(msg.contains("Fatal internal script panic"));
            }
            other => panic!("Expected EngineError::PanicTrapped, got: {other:?}"),
        }
        "safe-fallback-dom-layout".to_string()
    });

    assert_eq!(result, "safe-fallback-dom-layout");
}

#[test]
fn test_subsystem_profiles_conform_to_prd003() {
    let dom = SubsystemProfile::dom_parser();
    assert!(dom.contains(Capability::DOM_READ));
    assert!(dom.contains(Capability::DOM_MUTATE));
    assert!(!dom.contains(Capability::NETWORK_FETCH));
    assert!(!dom.contains(Capability::WINDOW_MANAGE));

    let css = SubsystemProfile::css_cascade();
    assert!(css.contains(Capability::DOM_READ));
    assert!(css.contains(Capability::GRAPHICS_DRAW));
    assert!(!css.contains(Capability::DOM_MUTATE));
    assert!(!css.contains(Capability::FS_READ_SCRIPTS));

    let net = SubsystemProfile::network_interceptor();
    assert!(net.contains(Capability::NETWORK_FETCH));
    assert!(net.contains(Capability::FS_WRITE_CACHE));
    assert!(!net.contains(Capability::WINDOW_MANAGE));
    assert!(!net.contains(Capability::GRAPHICS_DRAW));

    let ui = SubsystemProfile::ui_window();
    assert!(ui.contains(Capability::WINDOW_MANAGE));
    assert!(ui.contains(Capability::GRAPHICS_DRAW));
    assert!(ui.contains(Capability::DOM_READ));
    assert!(!ui.contains(Capability::NETWORK_LISTEN));
}
