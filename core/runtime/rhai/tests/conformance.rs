//! `RhaiEngine` must pass the same backend-agnostic suite as the `MockEngine`
//! reference adapter (ADR-0011 item 6). This is the F2 half of **C-02**
//! ("trait compliance tests").

use rhai_runtime::RhaiEngine;

#[test]
fn rhai_engine_passes_core_conformance() {
    engine::conformance::run_core_suite(RhaiEngine::new);
}
