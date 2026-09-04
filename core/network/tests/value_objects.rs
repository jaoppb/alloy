//! Domain value objects of `core/network`: `Url` parsing and its typed
//! refusals, `HeaderMap` case-insensitivity, `StatusCode` predicates, `Method`
//! body rules. Pure — no socket, no feature gate.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use network::{HeaderMap, HeaderName, HeaderValue, Method, NetworkError, Scheme, StatusCode, Url};

#[test]
fn a_well_formed_absolute_url_parses_into_its_parts() {
    let url = Url::parse("https://example.com:8443/a/b?q=1").unwrap();
    assert_eq!(url.scheme(), Scheme::Https);
    assert_eq!(url.host().to_string(), "example.com");
    assert_eq!(url.path().to_string(), "/a/b");
    assert!(url.query().is_some());
}

#[test]
fn a_url_with_no_host_is_a_typed_refusal() {
    assert!(matches!(
        Url::parse("http:///just-a-path"),
        Err(NetworkError::InvalidUrl { .. })
    ));
}

#[test]
fn a_scheme_the_engine_does_not_speak_is_a_typed_refusal() {
    assert!(matches!(
        Url::parse("ftp://example.com/file"),
        Err(NetworkError::InvalidUrl { .. })
    ));
}

#[test]
fn header_names_are_matched_case_insensitively() {
    let mut headers = HeaderMap::new();
    headers.set(
        HeaderName::new("Content-Type").unwrap(),
        HeaderValue::from_text("text/html").unwrap(),
    );
    assert_eq!(
        headers.text(&HeaderName::new("CONTENT-TYPE").unwrap()),
        Some("text/html")
    );
    assert!(headers.contains(&HeaderName::new("content-type").unwrap()));
}

#[test]
fn a_repeated_field_line_combines_with_a_comma() {
    let mut headers = HeaderMap::new();
    let name = HeaderName::new("accept").unwrap();
    headers.append(name.clone(), HeaderValue::from_text("text/html").unwrap());
    headers.append(
        name.clone(),
        HeaderValue::from_text("application/xml").unwrap(),
    );
    assert_eq!(headers.text(&name), Some("text/html, application/xml"));
}

#[test]
fn status_code_predicates_classify_the_ranges() {
    assert!(StatusCode::OK.is_success());
    assert!(StatusCode::new(301).unwrap().is_redirect());
    assert!(StatusCode::new(404).unwrap().is_client_error());
    assert!(StatusCode::new(503).unwrap().is_server_error());
    assert!(StatusCode::new(999).is_err());
}

#[test]
fn head_and_304_forbid_a_response_body() {
    assert!(!Method::Head.allows_response_body());
    assert!(Method::Get.allows_response_body());
    assert!(StatusCode::new(304).unwrap().forbids_body());
}
