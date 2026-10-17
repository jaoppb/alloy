//! Conformance tests for [`OmniboxClassifier`], [`OmniboxDestination`] and [`SearchTemplate`].
//!
//! Covers explicit schemes, schemeless hostnames, local addresses, multi-word queries,
//! special character percent-encoding, and runtime template replacement.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use network::{
    NetworkError, OmniboxClassifier, OmniboxDestination, SearchTemplate, Url, UrlDefect,
};

fn classifier() -> OmniboxClassifier {
    OmniboxClassifier::default()
}

// ---- Explicit schemes ------------------------------------------------------

#[test]
fn explicit_schemes_are_resolved_directly() {
    let resolved = classifier()
        .classify("https://news.ycombinator.com/")
        .unwrap();
    assert!(resolved.is_direct_url());
    assert!(!resolved.is_search_query());
    assert_eq!(resolved.url().to_text(), "https://news.ycombinator.com/");

    let local_http = classifier().classify("http://localhost:3000/").unwrap();
    assert!(local_http.is_direct_url());
    assert_eq!(local_http.url().to_text(), "http://localhost:3000/");

    let uppercase = classifier().classify("HTTP://EXAMPLE.COM/PATH").unwrap();
    assert_eq!(uppercase.url().to_text(), "http://example.com/PATH");

    let ipv6 = classifier().classify("http://[::1]:8080/").unwrap();
    assert_eq!(ipv6.url().to_text(), "http://[::1]:8080/");
}

#[test]
fn explicit_scheme_drops_fragment_per_url_contract() {
    let destination = classifier()
        .classify("https://example.com/page?query=1#fragment")
        .unwrap();
    assert_eq!(
        destination.url().to_text(),
        "https://example.com/page?query=1"
    );
}

#[test]
fn explicit_scheme_with_embedded_credentials_is_a_typed_error() {
    let error = classifier()
        .classify("https://user:pass@example.com")
        .unwrap_err();
    assert!(matches!(
        error,
        NetworkError::InvalidUrl {
            defect: UrlDefect::EmbeddedCredentials,
            ..
        }
    ));
}

// ---- Hostnames without schemes ---------------------------------------------

#[test]
fn hostnames_without_schemes_normalize_to_https() {
    let destination = classifier().classify("example.com").unwrap();
    assert!(destination.is_direct_url());
    assert_eq!(destination.url().to_text(), "https://example.com/");

    let path_url = classifier().classify("crates.io/crates/winit").unwrap();
    assert!(path_url.is_direct_url());
    assert_eq!(path_url.url().to_text(), "https://crates.io/crates/winit");

    let with_port = classifier()
        .classify("sub.domain.org:8443/api?key=val")
        .unwrap();
    assert_eq!(
        with_port.url().to_text(),
        "https://sub.domain.org:8443/api?key=val"
    );
}

// ---- Local addresses -------------------------------------------------------

#[test]
fn local_addresses_normalize_to_http() {
    let with_port = classifier().classify("localhost:8080").unwrap();
    assert!(with_port.is_direct_url());
    assert_eq!(with_port.url().to_text(), "http://localhost:8080/");

    let bare = classifier().classify("localhost").unwrap();
    assert_eq!(bare.url().to_text(), "http://localhost/");

    let with_path = classifier().classify("localhost/test").unwrap();
    assert_eq!(with_path.url().to_text(), "http://localhost/test");

    let ipv4_local = classifier().classify("127.0.0.1:3000").unwrap();
    assert_eq!(ipv4_local.url().to_text(), "http://127.0.0.1:3000/");

    let ipv6_local = classifier().classify("[::1]:8080").unwrap();
    assert_eq!(ipv6_local.url().to_text(), "http://[::1]:8080/");
}

// ---- Multi-word search queries ---------------------------------------------

#[test]
fn multi_word_queries_are_percent_encoded_into_default_search_engine() {
    let query1 = classifier().classify("how to learn rust").unwrap();
    assert!(query1.is_search_query());
    assert!(!query1.is_direct_url());
    assert_eq!(
        query1.url().to_text(),
        "https://duckduckgo.com/?q=how+to+learn+rust"
    );

    let query2 = classifier().classify("rust async stream").unwrap();
    assert_eq!(
        query2.url().to_text(),
        "https://duckduckgo.com/?q=rust+async+stream"
    );

    let trimmed = classifier().classify("   spaces   around   ").unwrap();
    assert_eq!(
        trimmed.url().to_text(),
        "https://duckduckgo.com/?q=spaces+++around"
    );
}

// ---- Words without dots ----------------------------------------------------

#[test]
fn single_words_without_dots_are_classified_as_search_queries() {
    let single = classifier().classify("rust").unwrap();
    assert!(single.is_search_query());
    assert_eq!(single.url().to_text(), "https://duckduckgo.com/?q=rust");

    let weather = classifier().classify("weather").unwrap();
    assert_eq!(weather.url().to_text(), "https://duckduckgo.com/?q=weather");
}

// ---- Percent encoding and special characters -------------------------------

#[test]
fn special_characters_in_search_queries_are_safely_percent_encoded() {
    let cpp = classifier().classify("c++").unwrap();
    assert_eq!(cpp.url().to_text(), "https://duckduckgo.com/?q=c%2B%2B");

    let ampersand = classifier().classify("rust & go").unwrap();
    assert_eq!(
        ampersand.url().to_text(),
        "https://duckduckgo.com/?q=rust+%26+go"
    );

    let accent = classifier().classify("café").unwrap();
    assert_eq!(
        accent.url().to_text(),
        "https://duckduckgo.com/?q=caf%C3%A9"
    );

    let price = classifier().classify("price: $100").unwrap();
    assert_eq!(
        price.url().to_text(),
        "https://duckduckgo.com/?q=price%3A+%24100"
    );

    let question = classifier().classify("what? #hash").unwrap();
    assert_eq!(
        question.url().to_text(),
        "https://duckduckgo.com/?q=what%3F+%23hash"
    );
}

// ---- Search template replacement -------------------------------------------

#[test]
fn runtime_replacement_of_search_template() {
    let mut custom = classifier();
    let kagi = SearchTemplate::new("https://kagi.com/search?q={searchTerms}").unwrap();
    custom.set_search_template(kagi);
    assert_eq!(
        custom.search_template().as_str(),
        "https://kagi.com/search?q={searchTerms}"
    );

    let destination = custom.classify("alloy browser").unwrap();
    assert_eq!(
        destination.url().to_text(),
        "https://kagi.com/search?q=alloy+browser"
    );

    let brace_template = SearchTemplate::new("https://searx.be/search?q={}").unwrap();
    custom.set_search_template(brace_template);
    let brace_dest = custom.classify("rust lang").unwrap();
    assert_eq!(
        brace_dest.url().to_text(),
        "https://searx.be/search?q=rust+lang"
    );
}

#[test]
fn invalid_search_template_is_refused() {
    assert!(SearchTemplate::new("not_a_valid_url").is_err());
    assert!(SearchTemplate::new("ftp://unsupported.org?q={}").is_err());
}

// ---- Accessors, Display and fallback behavior ------------------------------

#[test]
fn omnibox_destination_accessors_and_display() {
    let direct = classifier().classify("example.com").unwrap();
    assert!(matches!(direct, OmniboxDestination::DirectUrl(_)));
    assert_eq!(direct.to_string(), "https://example.com/");
    let parsed: Url = direct.into_url();
    assert_eq!(parsed.to_text(), "https://example.com/");

    let search = classifier().classify("test query").unwrap();
    assert!(matches!(search, OmniboxDestination::SearchQuery(_)));
    assert_eq!(search.to_string(), "https://duckduckgo.com/?q=test+query");
}

#[test]
fn empty_or_whitespace_input_produces_empty_search_query() {
    let empty = classifier().classify("").unwrap();
    assert!(empty.is_search_query());
    assert_eq!(empty.url().to_text(), "https://duckduckgo.com/?q=");

    let whitespace = classifier().classify("   \t  ").unwrap();
    assert!(whitespace.is_search_query());
    assert_eq!(whitespace.url().to_text(), "https://duckduckgo.com/?q=");
}

#[test]
fn invalid_hostnames_fall_back_to_search_queries() {
    let double_dot = classifier().classify("foo..bar").unwrap();
    assert!(double_dot.is_search_query());
    assert_eq!(
        double_dot.url().to_text(),
        "https://duckduckgo.com/?q=foo..bar"
    );

    let leading_dot = classifier().classify(".test").unwrap();
    assert!(leading_dot.is_search_query());

    let trailing_dot = classifier().classify("test.").unwrap();
    assert!(trailing_dot.is_search_query());
}
