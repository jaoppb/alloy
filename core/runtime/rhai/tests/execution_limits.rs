//! **C-04**: a runaway script is aborted with
//! `EngineError::ExecutionLimitExceeded` rather than hanging the process.

use std::time::{Duration, Instant};

use engine::{CapabilitySet, EngineError, ExecutionLimit, ExecutionLimits, RuntimeEngine};
use rhai_runtime::RhaiEngine;

const RUNAWAY: &str = "let n = 0; while true { n += 1; }";

#[test]
fn an_infinite_loop_trips_the_operation_ceiling() {
    let engine = RhaiEngine::with_limits(ExecutionLimits::strict().with_max_operations(50_000));
    let mut context = engine
        .create_context(CapabilitySet::empty())
        .expect("context");

    let started = Instant::now();
    let outcome = engine.eval_value(&mut context, RUNAWAY);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the loop must be aborted promptly, not run for seconds"
    );
    assert_eq!(
        outcome,
        Err(EngineError::execution_limit_exceeded(
            ExecutionLimit::Operations
        )),
    );
}

#[test]
fn an_infinite_loop_trips_the_wall_clock_ceiling_when_operations_are_unbounded() {
    let engine = RhaiEngine::with_limits(
        ExecutionLimits::strict()
            .with_max_operations(0) // 0 == unbounded in rhai
            .with_max_duration(Duration::from_millis(50)),
    );
    let mut context = engine
        .create_context(CapabilitySet::empty())
        .expect("context");

    let started = Instant::now();
    let outcome = engine.eval_value(&mut context, RUNAWAY);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "must abort promptly"
    );
    assert_eq!(
        outcome,
        Err(EngineError::execution_limit_exceeded(
            ExecutionLimit::Duration
        )),
    );
}

#[test]
fn a_bounded_script_runs_to_completion_within_the_ceilings() {
    let engine = RhaiEngine::new();
    let mut context = engine
        .create_context(CapabilitySet::empty())
        .expect("context");

    let total: i64 = engine
        .eval(&mut context, "let s = 0; for i in 0..100 { s += i; } s")
        .expect("a bounded loop completes normally");
    assert_eq!(total, 4950);
}
