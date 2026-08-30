//! `RhaiEngine` passes the object-safe `dyn` companion suite (ADR-0013 /
//! ADR-0011 item 2) through `Box<dyn DynRuntimeEngine>`, exactly as it passes
//! the core suite.

use rhai_runtime::RhaiEngine;

#[test]
fn rhai_engine_passes_dyn_companion_conformance() {
    engine::conformance::run_dyn_suite(Box::new(RhaiEngine::new()));
}
