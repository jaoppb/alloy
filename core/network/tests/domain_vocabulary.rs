//! The rest of `core/network`'s `domain/`: URL resolution, path/authority
//! validation, the status/method/media-type predicates, the message value
//! objects and the typed-error vocabulary. Pure — no socket, no feature gate.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use network::{
    Authority, Body, Charset, DecodeDefect, FramingDefect, HeaderMap, HeaderName, HeaderValue,
    Host, HttpRequest, HttpResponse, MalformedPart, MediaType, Method, NetworkError, Path, Port,
    ProtocolPhase, Query, RedirectDefect, RequestTarget, Scheme, StatusCode, Url, UrlDefect,
    WireLimit,
};

fn url(raw: &str) -> Url {
    Url::parse(raw).unwrap()
}

// ---- Url ------------------------------------------------------------------

#[test]
fn a_default_port_is_elided_from_the_text_form_and_a_custom_one_is_kept() {
    assert_eq!(
        url("HTTP://Example.COM/a").to_text(),
        "http://example.com/a"
    );
    assert_eq!(
        url("https://example.com:8443/a?x=1").to_string(),
        "https://example.com:8443/a?x=1"
    );
}

#[test]
fn a_bare_authority_gets_the_root_path_and_a_leading_query_keeps_it() {
    assert_eq!(url("http://example.com").path().as_str(), "/");
    let with_query = url("http://example.com?q=1");
    assert_eq!(with_query.path().as_str(), "/");
    assert_eq!(with_query.query().unwrap().as_str(), "q=1");
}

#[test]
fn a_fragment_never_reaches_the_url() {
    assert_eq!(
        url("http://example.com/a#section").to_text(),
        "http://example.com/a"
    );
}

#[test]
fn origin_comparison_ignores_the_target_but_not_scheme_or_port() {
    let base = url("https://example.com/a");
    assert!(base.has_same_origin_as(&url("https://example.com/b?c")));
    assert!(!base.has_same_origin_as(&url("http://example.com/a")));
    assert!(!base.has_same_origin_as(&url("https://example.com:8443/a")));
}

#[test]
fn from_parts_round_trips_through_the_accessors() {
    let authority = Authority::new(Host::new("example.com").unwrap(), Port::new(81).unwrap());
    let target = RequestTarget::new(Path::new("/x").unwrap(), Some(Query::new("a=b").unwrap()));
    let built = Url::from_parts(Scheme::Http, authority.clone(), target.clone());
    assert_eq!(built.authority(), &authority);
    assert_eq!(built.target(), &target);
    assert_eq!(built.port().number(), 81);
    assert_eq!(built.to_text(), "http://example.com:81/x?a=b");
}

#[test]
fn url_parsing_names_the_precise_defect() {
    let defect = |raw: &str| match Url::parse(raw) {
        Err(NetworkError::InvalidUrl { defect, .. }) => defect,
        other => panic!("expected InvalidUrl for {raw}, got {other:?}"),
    };
    assert_eq!(defect("example.com/a"), UrlDefect::MissingScheme);
    assert_eq!(defect("ftp://example.com"), UrlDefect::UnsupportedScheme);
    assert_eq!(
        defect("http://user:pw@example.com"),
        UrlDefect::EmbeddedCredentials
    );
    assert_eq!(defect("http://exa mple.com"), UrlDefect::MalformedHost);
    assert_eq!(defect("http://example.com:0"), UrlDefect::MalformedPort);
    assert_eq!(defect("http://example.com:99999"), UrlDefect::MalformedPort);
    assert_eq!(defect("http://example.com:"), UrlDefect::MalformedPort);
    assert_eq!(defect("http://[::1"), UrlDefect::MalformedHost);
    assert_eq!(defect("http://[::1]x"), UrlDefect::MalformedHost);
    assert_eq!(defect("http://example.com/a b"), UrlDefect::MalformedPath);
    assert_eq!(defect("http://example.com/a?b c"), UrlDefect::MalformedPath);
}

#[test]
fn join_resolves_every_reference_form_against_the_base() {
    let base = url("https://example.com/dir/page?old=1");
    let join = |reference: &str| base.join(reference).unwrap().to_text();
    assert_eq!(join(""), "https://example.com/dir/page?old=1");
    assert_eq!(join("#frag"), "https://example.com/dir/page?old=1");
    assert_eq!(join("http://other.org/x"), "http://other.org/x");
    assert_eq!(
        join("//cdn.example.com/a.css"),
        "https://cdn.example.com/a.css"
    );
    assert_eq!(join("?new=2"), "https://example.com/dir/page?new=2");
    assert_eq!(join("/abs"), "https://example.com/abs");
    assert_eq!(join("sibling"), "https://example.com/dir/sibling");
    assert_eq!(join("../up"), "https://example.com/up");
    assert_eq!(join("  ./here?q  "), "https://example.com/dir/here?q");
}

#[test]
fn a_reference_that_cannot_resolve_is_a_redirect_phase_error() {
    let error = url("http://example.com/").join("/a b").unwrap_err();
    assert_eq!(error.phase(), ProtocolPhase::Redirect);
    assert!(matches!(
        error,
        NetworkError::InvalidUrl {
            defect: UrlDefect::MalformedPath,
            ..
        }
    ));
}

// ---- Path / Query / Authority ---------------------------------------------

#[test]
fn a_path_normalises_dot_segments_and_reports_its_directory() {
    assert_eq!(Path::new("/a/./b/../c").unwrap().as_str(), "/a/c");
    assert_eq!(Path::new("/a/b/..").unwrap().as_str(), "/a/");
    assert_eq!(Path::new("/a/.").unwrap().as_str(), "/a/");
    assert_eq!(Path::new("/../../x").unwrap().as_str(), "/x");
    assert_eq!(Path::new("/a/b/c").unwrap().directory(), "/a/b/");
    assert_eq!(Path::root().directory(), "/");
    assert_eq!(Path::root().to_string(), "/");
}

#[test]
fn a_path_must_be_absolute_and_free_of_forbidden_characters() {
    assert_eq!(Path::new("relative"), Err(UrlDefect::MalformedPath));
    for forbidden in ["/a#b", "/a\\b", "/a\"b", "/a<b", "/a\u{e9}", "/a\tb"] {
        assert_eq!(Path::new(forbidden), Err(UrlDefect::MalformedPath));
    }
    assert_eq!(Query::new("a|b"), Err(UrlDefect::MalformedPath));
    assert_eq!(Query::new("a=b&c=d").unwrap().to_string(), "a=b&c=d");
}

#[test]
fn a_request_target_prints_its_query_only_when_it_has_one() {
    let path = Path::new("/p").unwrap();
    assert_eq!(RequestTarget::new(path.clone(), None).to_string(), "/p");
    let with_query = RequestTarget::new(path, Some(Query::new("k=v").unwrap()));
    assert_eq!(with_query.to_string(), "/p?k=v");
    assert_eq!(with_query.query().unwrap().as_str(), "k=v");
}

#[test]
fn hosts_are_lowercased_and_validated() {
    assert_eq!(Host::new("EXAMPLE.org").unwrap().to_string(), "example.org");
    assert_eq!(Host::new(""), Err(UrlDefect::MissingHost));
    for forbidden in ["a b", "a/b", "a@b", "a:b", "caf\u{e9}"] {
        assert_eq!(Host::new(forbidden), Err(UrlDefect::MalformedHost));
    }
}

#[test]
fn ports_reject_zero_and_default_from_the_scheme() {
    assert_eq!(Port::new(0), Err(UrlDefect::MalformedPort));
    assert_eq!(Port::parse("abc"), Err(UrlDefect::MalformedPort));
    assert_eq!(Port::parse("8080").unwrap().to_string(), "8080");
    assert_eq!(Port::default_for(Scheme::Http).number(), 80);
    assert_eq!(Port::default_for(Scheme::Https).number(), 443);
}

#[test]
fn an_authority_omits_only_the_scheme_default_port_in_a_host_header() {
    let authority = Authority::new(Host::new("example.com").unwrap(), Port::new(443).unwrap());
    assert_eq!(authority.host().as_str(), "example.com");
    assert_eq!(authority.port().number(), 443);
    assert_eq!(authority.to_string(), "example.com:443");
    assert_eq!(authority.to_header_text(Scheme::Https), "example.com");
    assert_eq!(authority.to_header_text(Scheme::Http), "example.com:443");
}

// ---- Scheme / Method / Status / Phase -------------------------------------

#[test]
fn schemes_parse_case_insensitively_and_know_their_security() {
    assert_eq!(Scheme::parse("HtTpS"), Ok(Scheme::Https));
    assert_eq!(Scheme::parse("gopher"), Err(UrlDefect::UnsupportedScheme));
    assert!(Scheme::Https.is_secure());
    assert!(!Scheme::Http.is_secure());
    assert_eq!(Scheme::Http.default_port(), 80);
    assert_eq!(Scheme::Https.to_string(), "https");
}

#[test]
fn methods_know_body_and_redirect_rules() {
    let every = [
        (Method::Get, "GET"),
        (Method::Head, "HEAD"),
        (Method::Post, "POST"),
        (Method::Put, "PUT"),
        (Method::Delete, "DELETE"),
        (Method::Patch, "PATCH"),
        (Method::Options, "OPTIONS"),
        (Method::Trace, "TRACE"),
    ];
    for (method, text) in every {
        assert_eq!(method.to_string(), text);
        assert_eq!(method.allows_response_body(), method != Method::Head);
        let safe = matches!(method, Method::Get | Method::Head);
        assert_eq!(method.is_rewritten_on_redirect(), !safe);
    }
}

#[test]
fn status_codes_classify_and_refuse_out_of_range_values() {
    assert!(StatusCode::new(99).is_err());
    assert!(StatusCode::new(600).is_err());
    let error = StatusCode::new(600).unwrap_err();
    assert!(matches!(
        error,
        NetworkError::Malformed {
            part: MalformedPart::StatusLineCode,
            ..
        }
    ));
    assert!(StatusCode::new(101).unwrap().is_informational());
    assert!(StatusCode::OK.is_success());
    assert!(StatusCode::FOUND.is_redirect());
    assert!(StatusCode::NOT_FOUND.is_client_error());
    assert!(StatusCode::new(503).unwrap().is_server_error());
    assert_eq!(StatusCode::NOT_FOUND.to_string(), "404");
    assert_eq!(StatusCode::OK.code(), 200);
}

#[test]
fn status_codes_carry_the_body_and_redirect_semantics_of_rfc_9110() {
    assert!(StatusCode::new(100).unwrap().forbids_body());
    assert!(StatusCode::new(204).unwrap().forbids_body());
    assert!(StatusCode::NOT_MODIFIED.forbids_body());
    assert!(!StatusCode::OK.forbids_body());

    for followable in [301, 302, 303, 307, 308] {
        assert!(
            StatusCode::new(followable)
                .unwrap()
                .is_followable_redirect()
        );
    }
    for not_followable in [200, 300, 304, 305, 306] {
        assert!(
            !StatusCode::new(not_followable)
                .unwrap()
                .is_followable_redirect()
        );
    }
    assert!(StatusCode::MOVED_PERMANENTLY.rewrites_method());
    assert!(StatusCode::SEE_OTHER.rewrites_method());
    assert!(!StatusCode::TEMPORARY_REDIRECT.rewrites_method());
    assert!(!StatusCode::PERMANENT_REDIRECT.rewrites_method());
}

#[test]
fn every_protocol_phase_has_a_distinct_name() {
    let phases = [
        (ProtocolPhase::Dns, "dns"),
        (ProtocolPhase::Connect, "connect"),
        (ProtocolPhase::Handshake, "handshake"),
        (ProtocolPhase::Header, "header"),
        (ProtocolPhase::Body, "body"),
        (ProtocolPhase::Redirect, "redirect"),
        (ProtocolPhase::Decode, "decode"),
    ];
    for (phase, name) in phases {
        assert_eq!(phase.name(), name);
        assert_eq!(phase.to_string(), name);
    }
}

// ---- MediaType / Charset ---------------------------------------------------

#[test]
fn charset_labels_fold_into_the_two_supported_encodings() {
    for utf8 in ["UTF-8", "utf8", " \"ascii\" ", "us-ascii"] {
        assert_eq!(Charset::from_label(utf8).unwrap(), Charset::Utf8);
    }
    for latin in ["latin1", "ISO-8859-1", "cp1252", "l1"] {
        assert_eq!(Charset::from_label(latin).unwrap(), Charset::Windows1252);
    }
    assert!(matches!(
        Charset::from_label("shift_jis"),
        Err(NetworkError::Decode {
            defect: DecodeDefect::UnsupportedCharset,
            ..
        })
    ));
    assert_eq!(Charset::Utf8.to_string(), "utf-8");
    assert_eq!(Charset::Windows1252.label(), "windows-1252");
}

#[test]
fn a_media_type_parses_lowercases_and_prints_back() {
    let parsed = MediaType::parse("Text/HTML ; Foo=bar; charset=\"UTF-8\"").unwrap();
    assert_eq!(parsed.type_name(), "text");
    assert_eq!(parsed.subtype(), "html");
    assert_eq!(parsed.charset(), Some(Charset::Utf8));
    assert_eq!(parsed.essence(), "text/html");
    assert_eq!(parsed.to_string(), "text/html; charset=utf-8");
    assert_eq!(
        MediaType::parse("image/png").unwrap().to_string(),
        "image/png"
    );
    assert_eq!(MediaType::new("Text", "CSS", None).essence(), "text/css");
}

#[test]
fn a_media_type_without_a_charset_parameter_has_none() {
    assert_eq!(
        MediaType::parse("text/plain; format=flowed")
            .unwrap()
            .charset(),
        None
    );
}

#[test]
fn a_malformed_media_type_or_unsupported_charset_is_a_typed_decode_error() {
    for bad in ["plain", "/html", "text/", ""] {
        assert!(
            matches!(
                MediaType::parse(bad),
                Err(NetworkError::Decode {
                    defect: DecodeDefect::MalformedMediaType,
                    ..
                })
            ),
            "{bad:?}"
        );
    }
    assert!(matches!(
        MediaType::parse("text/html; charset=klingon"),
        Err(NetworkError::Decode {
            defect: DecodeDefect::UnsupportedCharset,
            ..
        })
    ));
}

#[test]
fn only_text_like_media_types_are_textual() {
    for textual in [
        "text/css",
        "application/json",
        "application/xhtml+xml",
        "application/javascript",
    ] {
        assert!(MediaType::parse(textual).unwrap().is_textual(), "{textual}");
    }
    for binary in ["image/png", "application/octet-stream", "application/pdf"] {
        assert!(!MediaType::parse(binary).unwrap().is_textual(), "{binary}");
    }
}

// ---- Headers ---------------------------------------------------------------

#[test]
fn header_names_are_tokens_and_lowercased() {
    assert_eq!(
        HeaderName::new("X-Custom_1").unwrap().as_str(),
        "x-custom_1"
    );
    assert!(HeaderName::new("").is_err());
    assert!(HeaderName::new("bad name").is_err());
    assert!(HeaderName::new("bad:name").is_err());
}

#[test]
fn the_well_known_header_names_spell_their_wire_form() {
    let known = [
        (HeaderName::host(), "host"),
        (HeaderName::content_length(), "content-length"),
        (HeaderName::content_type(), "content-type"),
        (HeaderName::content_encoding(), "content-encoding"),
        (HeaderName::transfer_encoding(), "transfer-encoding"),
        (HeaderName::connection(), "connection"),
        (HeaderName::location(), "location"),
        (HeaderName::accept_encoding(), "accept-encoding"),
        (HeaderName::user_agent(), "user-agent"),
        (HeaderName::accept(), "accept"),
        (HeaderName::authorization(), "authorization"),
        (HeaderName::cookie(), "cookie"),
    ];
    for (name, wire) in known {
        assert_eq!(name.to_string(), wire);
        assert_eq!(HeaderName::new(wire).unwrap(), name);
    }
}

#[test]
fn header_values_refuse_line_breaks_and_trim_edge_whitespace() {
    for forbidden in [&b"a\r\nb"[..], b"a\rb", b"a\nb", b"a\0b"] {
        assert!(matches!(
            HeaderValue::parse(forbidden),
            Err(NetworkError::Malformed {
                part: MalformedPart::HeaderValue,
                ..
            })
        ));
    }
    let padded = HeaderValue::from_text("  text/html \t").unwrap();
    assert_eq!(padded.as_str(), Some("text/html"));
    assert_eq!(padded.len(), 9);
    assert!(HeaderValue::from_text("   ").unwrap().is_empty());
}

#[test]
fn a_non_utf8_header_value_keeps_its_bytes_and_prints_a_placeholder() {
    let value = HeaderValue::parse(&[0xE9, 0xE8]).unwrap();
    assert_eq!(value.as_str(), None);
    assert_eq!(value.as_bytes(), &[0xE9, 0xE8]);
    assert_eq!(value.to_string(), "<2 non-UTF-8 bytes>");
    assert_eq!(HeaderValue::from_text("ok").unwrap().to_string(), "ok");
}

#[test]
fn a_header_map_sets_replaces_removes_and_iterates() {
    let mut headers = HeaderMap::new();
    assert!(headers.is_empty());
    let name = HeaderName::new("x-one").unwrap();
    headers.set(name.clone(), HeaderValue::from_text("1").unwrap());
    headers.set(name.clone(), HeaderValue::from_text("2").unwrap());
    headers.set(HeaderName::accept(), HeaderValue::from_text("*/*").unwrap());
    assert_eq!(headers.len(), 2);
    assert_eq!(headers.get(&name).unwrap().as_str(), Some("2"));
    assert_eq!(headers.iter().count(), 2);
    assert!(headers.remove(&name));
    assert!(!headers.remove(&name));
    assert!(!headers.contains(&name));
    assert_eq!(headers.text(&name), None);
}

// ---- Body / Request / Response -------------------------------------------

#[test]
fn a_body_exposes_bytes_and_text_only_when_valid_utf8() {
    assert!(Body::empty().is_empty());
    let text = Body::from_text("héllo");
    assert_eq!(text.len(), 6);
    assert_eq!(text.as_str(), Some("héllo"));
    assert_eq!(text.to_string(), "<6 bytes>");
    let binary = Body::from_bytes(vec![0xFF, 0xFE]);
    assert_eq!(binary.as_str(), None);
    assert_eq!(binary.as_bytes(), &[0xFF, 0xFE]);
    assert_eq!(Body::from_slice(b"ab").as_bytes(), b"ab");
}

#[test]
fn a_request_builds_immutably_and_a_method_rewrite_drops_the_body() {
    let target = url("http://example.com/a");
    let request = HttpRequest::new(Method::Post, target.clone())
        .with_header(HeaderName::accept(), HeaderValue::from_text("*/*").unwrap())
        .with_body(Body::from_text("payload"));
    assert_eq!(request.method(), Method::Post);
    assert_eq!(request.url(), &target);
    assert_eq!(request.body().len(), 7);
    assert!(request.headers().contains(&HeaderName::accept()));
    assert_eq!(request.to_string(), "POST http://example.com/a");

    let moved = url("http://example.com/b");
    let rewritten = request
        .with_url(moved.clone())
        .without_header(&HeaderName::accept())
        .rewritten_to(Method::Get);
    assert_eq!(rewritten.method(), Method::Get);
    assert_eq!(rewritten.url(), &moved);
    assert!(rewritten.body().is_empty());
    assert!(rewritten.headers().is_empty());
    assert_eq!(HttpRequest::get(moved).method(), Method::Get);
}

#[test]
fn a_response_derives_its_media_type_from_content_type_and_ignores_a_bad_one() {
    let mut headers = HeaderMap::new();
    headers.set(
        HeaderName::content_type(),
        HeaderValue::from_text("text/html; charset=utf-8").unwrap(),
    );
    let response = HttpResponse::new(StatusCode::OK, headers, Body::from_text("<p>"));
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.media_type().unwrap().essence(), "text/html");
    assert_eq!(response.headers().len(), 1);
    assert_eq!(response.to_string(), "200 (<3 bytes>)");
    assert_eq!(response.clone().with_body(Body::empty()).body().len(), 0);
    assert_eq!(response.body().len(), 3);

    let mut broken = HeaderMap::new();
    broken.set(
        HeaderName::content_type(),
        HeaderValue::from_text("nonsense").unwrap(),
    );
    let unresolved = HttpResponse::new(StatusCode::OK, broken, Body::empty());
    assert!(unresolved.media_type().is_none());
}

// ---- Error vocabulary ------------------------------------------------------

#[test]
fn every_defect_reads_as_a_distinct_sentence_through_display() {
    let reasons: Vec<String> = [
        UrlDefect::MissingScheme,
        UrlDefect::UnsupportedScheme,
        UrlDefect::MissingHost,
        UrlDefect::MalformedHost,
        UrlDefect::MalformedPort,
        UrlDefect::MalformedPath,
        UrlDefect::EmbeddedCredentials,
        UrlDefect::UnresolvableReference,
    ]
    .iter()
    .map(ToString::to_string)
    .chain(
        [
            MalformedPart::StatusLineVersion,
            MalformedPart::StatusLineCode,
            MalformedPart::HeaderSeparator,
            MalformedPart::HeaderName,
            MalformedPart::HeaderValue,
            MalformedPart::HeaderNumber,
            MalformedPart::ContradictoryContentLength,
            MalformedPart::UnsupportedTransferEncoding,
            MalformedPart::TruncatedHead,
        ]
        .iter()
        .map(ToString::to_string),
    )
    .chain(
        [
            FramingDefect::BodyShorterThanDeclared,
            FramingDefect::ChunkSizeNotHexadecimal,
            FramingDefect::ChunkSizeMissing,
            FramingDefect::ChunkTerminatorMissing,
            FramingDefect::FinalChunkMissing,
            FramingDefect::ConnectionClosedEarly,
        ]
        .iter()
        .map(ToString::to_string),
    )
    .chain(
        [
            RedirectDefect::Cycle,
            RedirectDefect::LimitExceeded,
            RedirectDefect::LocationMissing,
            RedirectDefect::LocationUnresolvable,
        ]
        .iter()
        .map(ToString::to_string),
    )
    .chain(
        [
            DecodeDefect::UnsupportedContentEncoding,
            DecodeDefect::MalformedCompressedStream,
            DecodeDefect::CompressionRatioTooHigh,
            DecodeDefect::ChecksumMismatch,
            DecodeDefect::UnsupportedCharset,
            DecodeDefect::UnsupportedByteOrderMark,
            DecodeDefect::MalformedMediaType,
        ]
        .iter()
        .map(ToString::to_string),
    )
    .chain(
        [
            WireLimit::StatusLineLength,
            WireLimit::HeaderLineLength,
            WireLimit::HeaderCount,
            WireLimit::BodyLength,
            WireLimit::ChunkLineLength,
            WireLimit::DecodedLength,
        ]
        .iter()
        .map(ToString::to_string),
    )
    .collect();

    assert!(reasons.iter().all(|reason| !reason.is_empty()));
    let mut unique = reasons.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), reasons.len(), "two defects share a sentence");
}

#[test]
fn each_error_constructor_fixes_the_phase_it_belongs_to() {
    let cases = [
        (NetworkError::unresolved("h", "r"), ProtocolPhase::Dns),
        (
            NetworkError::unreachable("h:1", "r"),
            ProtocolPhase::Connect,
        ),
        (
            NetworkError::handshake_rejected("h", "r"),
            ProtocolPhase::Handshake,
        ),
        (
            NetworkError::framing(FramingDefect::ChunkSizeMissing),
            ProtocolPhase::Body,
        ),
        (
            NetworkError::redirect(RedirectDefect::Cycle),
            ProtocolPhase::Redirect,
        ),
        (
            NetworkError::decode(DecodeDefect::ChecksumMismatch),
            ProtocolPhase::Decode,
        ),
        (NetworkError::policy_denied("no"), ProtocolPhase::Dns),
        (
            NetworkError::timeout(ProtocolPhase::Header, 50),
            ProtocolPhase::Header,
        ),
        (
            NetworkError::transport(ProtocolPhase::Body, "reset"),
            ProtocolPhase::Body,
        ),
        (
            NetworkError::limit_exceeded(ProtocolPhase::Header, WireLimit::HeaderCount, 9),
            ProtocolPhase::Header,
        ),
        (
            NetworkError::malformed(ProtocolPhase::Header, MalformedPart::TruncatedHead),
            ProtocolPhase::Header,
        ),
        (
            NetworkError::invalid_url(ProtocolPhase::Dns, UrlDefect::MissingHost),
            ProtocolPhase::Dns,
        ),
    ];
    for (error, phase) in cases {
        assert_eq!(error.phase(), phase, "{error}");
        assert_eq!(error.phase_name(), phase.name());
    }
}

#[test]
fn in_phase_relabels_every_variant_without_touching_its_payload() {
    let original = NetworkError::timeout(ProtocolPhase::Header, 50);
    let moved = original.in_phase(ProtocolPhase::Body);
    assert_eq!(moved.phase(), ProtocolPhase::Body);
    assert_eq!(
        moved,
        NetworkError::timeout(ProtocolPhase::Body, 50),
        "only the phase differs"
    );
    let relabelled = NetworkError::unresolved("h", "r").in_phase(ProtocolPhase::Connect);
    assert_eq!(relabelled.phase(), ProtocolPhase::Connect);
}

#[test]
fn error_messages_name_the_phase_and_the_cause() {
    let message =
        NetworkError::limit_exceeded(ProtocolPhase::Body, WireLimit::BodyLength, 42).to_string();
    assert!(message.contains("body phase"), "{message}");
    assert!(message.contains("42 bytes"), "{message}");
    let timeout = NetworkError::timeout(ProtocolPhase::Connect, 1500).to_string();
    assert!(timeout.contains("1500 ms"), "{timeout}");
    let unreachable = NetworkError::unreachable("example.com:80", "refused").to_string();
    assert!(unreachable.contains("example.com:80") && unreachable.contains("refused"));
}
