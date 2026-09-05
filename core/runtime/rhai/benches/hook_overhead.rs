//! Criterion benchmark for Rhai hook dispatch overhead (Fase M, PRD-001:96, N-01).
//!
//! Measures the round-trip execution of an event hook (`on_event`) over a
//! pre-compiled AST. Performance target: p99 < 10 µs.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_docs_in_private_items
)]

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use engine::{CapabilitySet, EngineValue, ExecutionContext, RuntimeEngine, VariableName};
use rhai_runtime::RhaiEngine;

fn bench_on_event_hook(c: &mut Criterion) {
    let engine = RhaiEngine::new();
    let mut context = engine
        .create_context(CapabilitySet::empty())
        .expect("context creation");
    let script_source = r#"
        if event == "click" {
            1
        } else {
            0
        }
    "#;
    let compiled = engine.compile(script_source).expect("compile hook script");
    let event_var = VariableName::parse("event").expect("variable name");

    c.bench_function("on_event_overhead", |b| {
        b.iter(|| {
            context
                .set_variable(&event_var, EngineValue::Text("click".to_owned()))
                .expect("set event variable");
            let result = engine
                .eval_compiled_value(&mut context, &compiled)
                .expect("eval compiled hook");
            black_box(result);
        });
    });
}

criterion_group!(benches, bench_on_event_hook);
criterion_main!(benches);
