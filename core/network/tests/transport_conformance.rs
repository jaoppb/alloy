//! **`ADR-0011` item 6 / `PRD-009`**: every [`HttpTransport`] adapter passes the
//! same backend-agnostic suite. `MockTransport` is the in-repo reference the
//! `no-transport` build runs; `RealHttpTransport` runs it too when
//! `real-transport` is linked.
//!
//! The hostile-response classes of `PRD-009` (a lying `Content-Length`, a bad
//! chunk, a giant header line) are pinned here through `MockTransport::parse_wire`
//! — the seam that turns a raw fixture into a response, so a malformed peer
//! fails typed instead of hanging or panicking.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use network::{
    Body, HeaderMap, HeaderName, HeaderValue, HttpResponse, HttpTransport, MockTransport,
    NetworkError, StatusCode, Url,
};

#[test]
fn the_empty_mock_transport_passes_the_conformance_suite() {
    network::conformance::run_transport_suite(&MockTransport::new());
}

#[test]
fn a_mock_transport_with_a_fixture_still_passes_the_conformance_suite() {
    let url = Url::parse("http://probe.conformance.invalid/probe").unwrap();
    let mut headers = HeaderMap::new();
    headers.set(
        HeaderName::content_type(),
        HeaderValue::from_text("text/plain; charset=utf-8").unwrap(),
    );
    let response = HttpResponse::new(StatusCode::OK, headers, Body::from_text("ok"));
    let transport = MockTransport::new().with_response(url, response);

    network::conformance::run_transport_suite(&transport);
}

#[test]
fn a_head_request_is_answered_without_a_body() {
    let url = Url::parse("http://probe.conformance.invalid/probe").unwrap();
    let response = HttpResponse::new(StatusCode::OK, HeaderMap::new(), Body::from_text("body"));
    let transport = MockTransport::new().with_response(url.clone(), response);

    let head = network::HttpRequest::new(network::Method::Head, url);
    let answered = transport.execute(&head).unwrap();
    assert!(answered.body().is_empty(), "a HEAD answer carries no body");
}

#[test]
fn a_lying_content_length_is_a_typed_framing_error_not_a_hang() {
    // Declares 100 body bytes, sends 3. A total transport must reject this,
    // typed, well within the harness timeout — never block waiting for bytes
    // that will not come.
    let wire = b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nabc";
    let outcome = MockTransport::parse_wire(wire);
    assert!(
        matches!(
            outcome,
            Err(NetworkError::Framing { .. } | NetworkError::Malformed { .. })
        ),
        "a short body under a large Content-Length must be a typed Framing/Malformed error, got {outcome:?}"
    );
}

#[test]
fn an_invalid_chunk_size_line_is_a_typed_error() {
    let wire = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nZZZZ\r\nabc\r\n0\r\n\r\n";
    let outcome = MockTransport::parse_wire(wire);
    assert!(
        matches!(
            outcome,
            Err(NetworkError::Framing { .. } | NetworkError::Malformed { .. })
        ),
        "a non-hex chunk size must be a typed error, got {outcome:?}"
    );
}

#[test]
fn a_giant_header_line_is_refused_by_the_wire_limits() {
    let mut wire = Vec::from(*b"HTTP/1.1 200 OK\r\nX-Huge: ");
    wire.extend(std::iter::repeat_n(b'a', 2 * 1024 * 1024));
    wire.extend_from_slice(b"\r\n\r\n");
    let outcome = MockTransport::parse_wire(&wire);
    assert!(
        matches!(
            outcome,
            Err(NetworkError::LimitExceeded { .. } | NetworkError::Malformed { .. })
        ),
        "a 2 MiB header line must trip the wire limits, got {:?}",
        outcome.map(|_| "unexpected Ok")
    );
}
