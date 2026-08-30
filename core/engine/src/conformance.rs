//! Backend-agnostic conformance suite (ADR-0011 item 6).
//!
//! Every adapter of the [`RuntimeEngine`] port must pass [`run_core_suite`]. It
//! is ordinary library code (not `#[cfg(test)]`) so any adapter crate can call
//! it from its own `tests/`:
//!
//! ```text
//! #[test]
//! fn conforms() {
//!     engine::conformance::run_core_suite(MyEngine::new);
//! }
//! ```
//!
//! The `MockEngine` reference adapter in `core/engine/tests/` and the real
//! `RhaiEngine` in `core/runtime/rhai` (F2) both run this exact suite.
//!
//! Checks that need a live interpreter (the infinite-loop / `ExecutionLimit`
//! breach of C-04, panic trapping of C-09) live in each adapter's own tests —
//! `MockEngine` has no loops to run away.
//!
//! ## What `call_function` means in v0.1
//!
//! [`ExecutionContext::call_function`] (and `call_function_value`) invokes a
//! **registered native binding** by name — the same handler `register_fn` /
//! `register_native_fn` installed. Invoking a function *defined by a compiled
//! script* (the `on_init` / `on_event` / `on_process` / `on_reload` hook
//! lifecycle of PRD-001 §5.2, needed by hot-reload) is **not** expressible
//! through the v0.1 port: the signature carries no compiled AST. That is a known
//! gap, tracked for v0.2 with a PRD-002 amendment. [`check_call_function_invokes_a_registered_binding`]
//! pins the current meaning so the semantics cannot drift silently.

use crate::application::ports::{ExecutionContext, RuntimeEngine};
use crate::domain::capability::{Capability, CapabilitySet};
use crate::domain::value::EngineValue;

/// Run every core check against engines produced by `make_engine`. Panics with a
/// descriptive message on the first violation.
pub fn run_core_suite<Engine, Make>(make_engine: Make)
where
    Engine: RuntimeEngine,
    Make: Fn() -> Engine,
{
    check_literal_evaluation(&make_engine());
    check_typed_eval_sugar(&make_engine());
    check_variable_roundtrip(&make_engine());
    check_compiled_path_matches_source_path(&make_engine());
    check_invalid_source_is_a_compilation_error(&make_engine());
    check_contexts_are_isolated(&make_engine());
    check_native_function_dispatch(&make_engine());
    check_call_function_invokes_a_registered_binding(&make_engine());
    check_call_function_rejects_an_unknown_name(&make_engine());
    check_capabilities_are_carried(&make_engine());
    check_reset_scope_clears_locals(&make_engine());
}

fn context<Engine: RuntimeEngine>(engine: &Engine) -> Engine::Context {
    engine
        .create_context(CapabilitySet::empty())
        .unwrap_or_else(|error| panic!("create_context(empty) must succeed, got {error}"))
}

fn check_literal_evaluation<Engine: RuntimeEngine>(engine: &Engine) {
    let mut scope = context(engine);
    let value = engine
        .eval_value(&mut scope, "1")
        .unwrap_or_else(|error| panic!("eval_value(\"1\") failed: {error}"));
    assert_eq!(
        value,
        EngineValue::Int(1),
        "integer literal must evaluate to Int(1)"
    );

    let flag = engine
        .eval_value(&mut scope, "true")
        .unwrap_or_else(|error| panic!("eval_value(\"true\") failed: {error}"));
    assert_eq!(
        flag,
        EngineValue::Bool(true),
        "boolean literal must evaluate to Bool(true)"
    );
}

fn check_typed_eval_sugar<Engine: RuntimeEngine>(engine: &Engine) {
    let mut scope = context(engine);
    let answer: i64 = engine
        .eval(&mut scope, "42")
        .unwrap_or_else(|error| panic!("eval::<i64>(\"42\") failed: {error}"));
    assert_eq!(answer, 42, "typed eval sugar must convert Int -> i64");
}

fn check_variable_roundtrip<Engine: RuntimeEngine>(engine: &Engine) {
    let mut scope = context(engine);
    scope
        .set_variable("answer", 42_i64)
        .unwrap_or_else(|error| panic!("set_variable failed: {error}"));
    let seen: i64 = engine
        .eval(&mut scope, "answer")
        .unwrap_or_else(|error| panic!("reading a set variable back failed: {error}"));
    assert_eq!(
        seen, 42,
        "a variable set from Rust must be visible to the script"
    );
}

fn check_compiled_path_matches_source_path<Engine: RuntimeEngine>(engine: &Engine) {
    let mut scope = context(engine);
    let compiled = engine
        .compile("7")
        .unwrap_or_else(|error| panic!("compile(\"7\") failed: {error}"));
    let from_compiled = engine
        .eval_compiled_value(&mut scope, &compiled)
        .unwrap_or_else(|error| panic!("eval_compiled_value failed: {error}"));
    let from_source = engine
        .eval_value(&mut scope, "7")
        .unwrap_or_else(|error| panic!("eval_value failed: {error}"));
    assert_eq!(
        from_compiled, from_source,
        "compiled and source evaluation of the same program must agree"
    );
}

fn check_invalid_source_is_a_compilation_error<Engine: RuntimeEngine>(engine: &Engine) {
    // Note: no `Debug` bound on `Engine::CompiledScript` — match instead of `expect_err`.
    let error = match engine.compile("@") {
        Ok(_) => panic!("compiling nonsense (\"@\") must fail, not produce a program"),
        Err(error) => error,
    };
    assert!(
        matches!(error, crate::EngineError::Compilation { .. }),
        "a syntax failure must be EngineError::Compilation, got {error:?}"
    );
}

fn check_contexts_are_isolated<Engine: RuntimeEngine>(engine: &Engine) {
    let mut first = context(engine);
    let mut second = context(engine);
    first
        .set_variable("x", 1_i64)
        .expect("set x in first context");
    second
        .set_variable("x", 2_i64)
        .expect("set x in second context");

    let from_first: i64 = engine.eval(&mut first, "x").expect("read x from first");
    let from_second: i64 = engine.eval(&mut second, "x").expect("read x from second");
    assert_eq!(from_first, 1, "first context keeps its own x");
    assert_eq!(
        from_second, 2,
        "second context is not disturbed by the first"
    );
}

fn check_native_function_dispatch<Engine: RuntimeEngine>(engine: &Engine) {
    let mut scope = context(engine);
    scope
        .register_fn("meaning", || 42_i64)
        .unwrap_or_else(|error| panic!("register_fn failed: {error}"));
    let called: i64 = engine
        .eval(&mut scope, "meaning()")
        .unwrap_or_else(|error| {
            panic!("calling a registered native fn from script failed: {error}")
        });
    assert_eq!(
        called, 42,
        "a registered native fn must be callable from the script"
    );
}

fn check_call_function_invokes_a_registered_binding<Engine: RuntimeEngine>(engine: &Engine) {
    let mut scope = context(engine);
    scope
        .register_fn("increment", |value: i64| value + 1)
        .unwrap_or_else(|error| panic!("register_fn failed: {error}"));
    let result: i64 = scope
        .call_function("increment", &[EngineValue::Int(41)])
        .unwrap_or_else(|error| panic!("call_function on a registered binding failed: {error}"));
    assert_eq!(
        result, 42,
        "call_function must invoke the registered native binding by name"
    );
}

fn check_call_function_rejects_an_unknown_name<Engine: RuntimeEngine>(engine: &Engine) {
    let mut scope = context(engine);
    let outcome = scope.call_function::<EngineValue>("no_such_binding", &[]);
    assert!(
        matches!(outcome, Err(crate::EngineError::Binding { .. })),
        "call_function on an unregistered name must be EngineError::Binding, got {outcome:?}"
    );
}

fn check_capabilities_are_carried<Engine: RuntimeEngine>(engine: &Engine) {
    let granted = CapabilitySet::new(Capability::DOM_READ | Capability::DOM_MUTATE);
    let scope = engine
        .create_context(granted)
        .unwrap_or_else(|error| panic!("create_context with a grant failed: {error}"));
    assert!(
        scope.capabilities().contains(Capability::DOM_READ),
        "the context must report the capabilities it was built with"
    );
    assert!(
        !scope.capabilities().contains(Capability::NETWORK_FETCH),
        "the context must not report capabilities it was never granted"
    );
}

fn check_reset_scope_clears_locals<Engine: RuntimeEngine>(engine: &Engine) {
    let mut scope = context(engine);
    scope.set_variable("temp", 99_i64).expect("set temp");
    scope
        .reset_scope()
        .unwrap_or_else(|error| panic!("reset_scope failed: {error}"));
    let after = engine.eval::<EngineValue>(&mut scope, "temp");
    assert!(
        after.is_err(),
        "after reset_scope a script-local variable must no longer resolve, got {after:?}"
    );
}
