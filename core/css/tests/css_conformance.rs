//! Guards the `PRD-007` §4 port contract: `run_css_conformance` must pass for
//! **both** the built-in Rust adapters and the mocks — proving the suite pins
//! the port, not one implementation.

use css::conformance::run_css_conformance;
use css::{BlockLayout, MockCascadeResolver, MockLayoutEngine, UaCascade};

#[test]
fn the_builtin_rust_adapters_pass_conformance() {
    run_css_conformance(&UaCascade::new(), &BlockLayout::new());
}

#[test]
fn the_port_mocks_pass_conformance() {
    run_css_conformance(&MockCascadeResolver::new(), &MockLayoutEngine::new());
}
