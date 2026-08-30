//! Mechanism of **C-09**: a panic inside script execution — including inside a
//! registered native function — is trapped and returned as
//! `EngineError::ScriptPanic`; the host process stays alive. (The full
//! DevTools-logging fallback handler is F6/v0.2.)

use engine::{CapabilitySet, EngineError, ExecutionContext, RuntimeEngine};
use rhai_runtime::RhaiEngine;

#[test]
fn a_panicking_native_function_is_trapped_and_the_engine_stays_usable() {
    let engine = RhaiEngine::new();
    let mut context = engine
        .create_context(CapabilitySet::empty())
        .expect("context");
    context
        .register_fn("boom", || -> i64 { panic!("native code exploded") })
        .expect("register boom");

    match engine.eval_value(&mut context, "boom()") {
        Err(EngineError::ScriptPanic { message }) => {
            assert!(
                message.contains("native code exploded"),
                "message was: {message}"
            );
        }
        other => panic!("expected a trapped ScriptPanic, got {other:?}"),
    }

    // Same context, same engine: still works after the trapped panic.
    let after: i64 = engine
        .eval(&mut context, "1 + 1")
        .expect("the engine is still usable after trapping a panic");
    assert_eq!(after, 2);
}

#[test]
fn a_runtime_error_carries_its_source_location() {
    let engine = RhaiEngine::new();
    let mut context = engine
        .create_context(CapabilitySet::empty())
        .expect("context");

    match engine.eval_value(&mut context, "\n\nmissing_variable") {
        Err(EngineError::ScriptRuntime { location, .. }) => {
            let position = location.expect("a runtime error should carry a location");
            assert_eq!(
                position.line().get(),
                3,
                "the reference is on the third line"
            );
        }
        other => panic!("expected ScriptRuntime, got {other:?}"),
    }
}
