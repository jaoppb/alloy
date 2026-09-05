//! Tests for network bindings and request policy (Fase M, PRD-009, C-06, C-07).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::significant_drop_tightening
)]

use engine::{Capability, CapabilitySet, EngineError, RuntimeEngine, profiles};
use network::{HttpRequest, PolicyVerdict, RequestPolicy, Url};
use rhai_bindings::{
    DEFAULT_NETWORK_SCRIPT, NETWORK_BINDINGS, ScriptRequestPolicy, register_net_bindings,
};
use rhai_runtime::RhaiEngine;

#[test]
fn ui_script_without_network_fetch_is_denied_when_fetching() {
    let engine = RhaiEngine::new();
    let ui_caps = profiles::ui_window(); // WINDOW_MANAGE | GRAPHICS_DRAW | DOM_READ (no NETWORK_FETCH)
    let mut context = engine.create_context(ui_caps).expect("context");
    register_net_bindings(&mut context).expect("register_net_bindings");

    let outcome = engine.eval_value(&mut context, r#"fetch("https://example.com")"#);
    match outcome {
        Err(EngineError::PermissionDenied { capability }) => {
            assert_eq!(capability, Capability::NETWORK_FETCH);
        }
        other => panic!("expected PermissionDenied(NETWORK_FETCH), got {other:?}"),
    }
}

#[test]
fn every_network_binding_is_denied_without_network_fetch() {
    let engine = RhaiEngine::new();
    let empty_caps = CapabilitySet::empty();
    let mut context = engine.create_context(empty_caps).expect("context");
    register_net_bindings(&mut context).expect("register_net_bindings");

    let snippets = [
        ("fetch", r#"fetch("https://example.com")"#),
        ("allow", r#"allow("request")"#),
        ("deny", r#"deny("tracking")"#),
        ("rewrite", r#"rewrite("https://example.com")"#),
        ("header", r#"header("user-agent", "Alloy")"#),
    ];

    let declared: Vec<&str> = NETWORK_BINDINGS.iter().map(|(name, _)| *name).collect();
    let covered: Vec<&str> = snippets.iter().map(|(name, _)| *name).collect();
    assert_eq!(declared, covered);

    for (name, snippet) in snippets {
        let outcome = engine.eval_value(&mut context, snippet);
        assert!(
            matches!(
                outcome,
                Err(EngineError::PermissionDenied {
                    capability: Capability::NETWORK_FETCH
                })
            ),
            "{name}: expected PermissionDenied(NETWORK_FETCH), got {outcome:?}"
        );
    }
}

#[test]
fn network_bindings_execute_under_network_fetch() {
    let engine = RhaiEngine::new();
    let caps = profiles::network_interceptor();
    let mut context = engine.create_context(caps).expect("context");
    register_net_bindings(&mut context).expect("register_net_bindings");

    let allow_outcome = engine
        .eval_value(&mut context, r#"allow("req")"#)
        .expect("allow");
    assert!(matches!(allow_outcome, engine::EngineValue::Map(_)));

    let deny_outcome = engine
        .eval_value(&mut context, r#"deny("ad tracker")"#)
        .expect("deny");
    assert!(matches!(deny_outcome, engine::EngineValue::Map(_)));

    let rewrite_outcome = engine
        .eval_value(&mut context, r#"rewrite("https://safe.example.com")"#)
        .expect("rewrite");
    assert!(matches!(rewrite_outcome, engine::EngineValue::Map(_)));

    let header_outcome = engine
        .eval_value(&mut context, r#"header("authorization", "Bearer xyz")"#)
        .expect("header");
    assert!(matches!(header_outcome, engine::EngineValue::Map(_)));
}

#[test]
fn default_network_policy_allows_clean_requests() {
    let engine = RhaiEngine::new();
    let policy = ScriptRequestPolicy::new(engine, DEFAULT_NETWORK_SCRIPT);

    let url = Url::parse("https://example.com").expect("url");
    let request = HttpRequest::get(url);
    let verdict = policy.decide(&request).expect("decide");
    assert_eq!(verdict, PolicyVerdict::Allow);
}

#[test]
fn custom_network_script_policy_can_deny_and_rewrite() {
    let engine = RhaiEngine::new();
    let script = r#"
        if request.contains("adservice") {
            deny("tracker blocked")
        } else if request.starts_with("http://") {
            rewrite("https://upgrade.example.com")
        } else {
            allow(request)
        }
    "#;
    let policy = ScriptRequestPolicy::new(engine, script);

    let ad_url = Url::parse("https://adservice.example.com/track").expect("url");
    let ad_request = HttpRequest::get(ad_url);
    let ad_verdict = policy.decide(&ad_request).expect("decide");
    assert_eq!(
        ad_verdict,
        PolicyVerdict::Deny {
            reason: "tracker blocked".to_owned()
        }
    );

    let http_url = Url::parse("http://example.com/page").expect("url");
    let http_request = HttpRequest::get(http_url);
    let http_verdict = policy.decide(&http_request).expect("decide");
    let expected_url = Url::parse("https://upgrade.example.com").expect("url");
    assert_eq!(
        http_verdict,
        PolicyVerdict::Rewrite(http_request.with_url(expected_url))
    );
}
