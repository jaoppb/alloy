//! Domain value objects of `core/network`: `Url` parsing and its typed
//! refusals, `HeaderMap` case-insensitivity, `StatusCode` predicates, `Method`
//! body rules. Pure — no socket, no feature gate.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use network::{
    HeaderMap, HeaderName, HeaderValue, HttpVersion, MediaType, MediaTypeName, Method,
    NetworkError, Query, Scheme, StatusCode, Url,
};

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

#[test]
fn query_is_safe_and_idempotent_like_get_on_redirect() {
    assert!(!Method::Query.is_rewritten_on_redirect());
    assert_eq!(Method::Query.as_str(), "QUERY");
}

#[test]
fn a_query_string_parses_into_ordered_decoded_pairs() {
    let query = Query::new("a=1&flag&b=hello%20world").unwrap();
    let mut params = query.params();
    let first = params.next().unwrap();
    assert_eq!(first.key(), "a");
    assert_eq!(first.value(), "1");
    let second = params.next().unwrap();
    assert_eq!(second.key(), "flag");
    assert_eq!(second.value(), "");
    assert_eq!(query.get("b"), Some("hello world"));
    assert_eq!(query.as_str(), "a=1&flag&b=hello%20world");
}

#[test]
fn a_plus_sign_decodes_to_a_space_per_whatwg_urlencoded_parsing() {
    let query = Query::new("q=hello+world").unwrap();
    assert_eq!(query.get("q"), Some("hello world"));
}

#[test]
fn a_truncated_percent_escape_passes_through_literally_per_whatwg_urlencoded_parsing() {
    let query = Url::parse("https://example.com/?a=%2").unwrap();
    assert_eq!(query.query().and_then(|query| query.get("a")), Some("%2"));
}

#[test]
fn a_non_hexadecimal_percent_escape_passes_through_literally() {
    let query = Query::new("a=100%25%zz").unwrap();
    assert_eq!(query.get("a"), Some("100%%zz"));
}

#[test]
fn an_empty_query_string_carries_no_pairs() {
    let query = Query::new("").unwrap();
    assert_eq!(query.params().count(), 0);
    assert_eq!(query.get("anything"), None);
    assert_eq!(query.as_str(), "");
}

#[test]
fn get_answers_the_first_pair_when_a_key_repeats() {
    let query = Query::new("a=1&a=2").unwrap();
    assert_eq!(query.get("a"), Some("1"));
    assert_eq!(query.params().count(), 2);
}

#[test]
fn a_relative_query_only_reference_resolves_against_the_base_path() {
    let base = Url::parse("https://example.com/a/b?old=1").unwrap();
    let resolved = base.join("?new=2").unwrap();
    assert_eq!(resolved.path().to_string(), "/a/b");
    assert_eq!(
        resolved.query().and_then(|query| query.get("new")),
        Some("2")
    );
}

#[test]
fn http_version_parses_only_the_two_versions_this_engine_speaks() {
    assert_eq!(HttpVersion::parse("HTTP/1.1"), Some(HttpVersion::Http11));
    assert_eq!(HttpVersion::parse("HTTP/1.0"), Some(HttpVersion::Http10));
    assert_eq!(HttpVersion::parse("HTTP/2"), None);
}

#[test]
fn media_type_name_lowercases_and_rejects_illegal_characters() {
    assert_eq!(MediaTypeName::new("TEXT").unwrap().as_str(), "text");
    assert!(MediaTypeName::new("").is_err());
    assert!(MediaTypeName::new("a/b").is_err());
}

#[test]
fn a_content_type_is_textual_only_for_the_known_type_subtype_pairs() {
    let html = MediaType::parse("text/html; charset=utf-8").unwrap();
    assert!(html.is_textual());
    let png = MediaType::parse("image/png").unwrap();
    assert!(!png.is_textual());
}
