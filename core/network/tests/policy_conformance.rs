//! **`ADR-0011` item 6 / `PRD-009`**: the [`RequestPolicy`] port. `AllowAllPolicy`
//! is the trivial reference the suite runs; Phase M installs the real `.rhai`
//! policy on the same seam.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use network::{AllowAllPolicy, HttpRequest, PolicyVerdict, RequestPolicy, Url};

#[test]
fn allow_all_policy_passes_the_conformance_suite() {
    network::conformance::run_policy_suite(&AllowAllPolicy::new());
}

#[test]
fn allow_all_permits_every_request() {
    let policy = AllowAllPolicy::new();
    let request = HttpRequest::get(Url::parse("https://example.invalid/x").unwrap());
    assert_eq!(policy.decide(&request).unwrap(), PolicyVerdict::Allow);
}

#[test]
fn a_denial_verdict_carries_its_reason() {
    let verdict = PolicyVerdict::deny("blocked by test");
    match verdict {
        PolicyVerdict::Deny { reason } => assert_eq!(reason, "blocked by test"),
        other => panic!("expected Deny, got {other:?}"),
    }
}
