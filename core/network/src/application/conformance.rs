//! A transport- and policy-agnostic conformance suite — `ADR-0011` item 6.
//!
//! Ordinary library code, not `#[cfg(test)]`, so an adapter can call it from
//! its own `tests/` — the same shape and reason as
//! `core/graphics/src/application/conformance.rs` and
//! `core/css/src/application/conformance.rs`, which their adapters both run.
//!
//! ```text
//! #[test]
//! fn my_transport_passes_conformance() {
//!     network::conformance::run_transport_suite(&MyTransport::new());
//! }
//! ```
//!
//! ## What it pins, and what it deliberately does not
//!
//! It pins the **port contract**: totality (an answer is a response or a typed
//! [`NetworkError`], never a panic), boundedness (an answer arrives, it does not
//! hang), re-entrancy (the same request twice gives the same answer), and
//! self-consistency (a response's resolved media type agrees with its own
//! header section).
//!
//! It does **not** pin an adapter's routing table. `MockTransport` serves a
//! fixture map and `RealHttpTransport` opens sockets; neither can be required to
//! answer `200` to a URL this file invented. A typed failure for a probe is a
//! **pass** — what would be a failure is a panic, a hang, or an answer that
//! contradicts itself. The hostile-response classes of `PRD-009` (a lying
//! `Content-Length`, a bad chunk, a giant header, a redirect cycle) are pinned
//! against a real server in `core/network/tests/hostile_responses.rs`, where a
//! fixture can actually be served.

// An assertion suite that happens to be `pub` (so adapters can call it from
// their `tests/`) rather than `#[cfg(test)]`: it panics on the first violation
// by design. Same carve-out, same reason, as
// `core/graphics/src/application/conformance.rs:29` and
// `core/css/src/application/conformance.rs:24`.
#![allow(clippy::panic, clippy::expect_used)]

use std::time::{Duration, Instant};

use crate::application::ports::{HttpTransport, PolicyVerdict, RequestPolicy};
use crate::domain::defect::UrlDefect;
use crate::domain::error::NetworkError;
use crate::domain::header_map::HeaderName;
use crate::domain::media_type::MediaType;
use crate::domain::method::Method;
use crate::domain::phase::ProtocolPhase;
use crate::domain::request::HttpRequest;
use crate::domain::response::HttpResponse;
use crate::domain::url::Url;

/// The reserved TLD RFC 2606 sets aside precisely so a probe can never reach a
/// real host by accident.
const PROBE_ORIGIN: &str = "http://probe.conformance.invalid";

/// How long one `execute` may take before the port contract is considered
/// broken. Generous: a real transport doing DNS plus a TLS handshake on a cold
/// cache is still an order of magnitude under this.
const ANSWER_BUDGET: Duration = Duration::from_secs(30);

/// How long one `decide` may take. A policy does no I/O, so this is enormous.
const VERDICT_BUDGET: Duration = Duration::from_secs(5);

/// Runs every rule an [`HttpTransport`] must obey.
///
/// Panics on the first violation, naming the rule that was broken.
pub fn run_transport_suite(transport: &dyn HttpTransport) {
    check_a_request_cannot_exist_without_an_authority();
    check_every_probe_is_answered_typed(transport);
    check_an_answer_is_self_consistent(transport);
    check_execute_is_re_entrant(transport);
    check_an_answer_arrives_within_the_budget(transport);
    check_a_pre_response_failure_names_a_pre_response_phase(transport);
    check_a_head_answer_carries_no_body(transport);
}

/// Runs every rule a [`RequestPolicy`] must obey.
///
/// Panics on the first violation, naming the rule that was broken.
pub fn run_policy_suite(policy: &dyn RequestPolicy) {
    check_every_probe_gets_a_verdict(policy);
    check_decide_is_pure(policy);
    check_a_denial_carries_a_reason(policy);
    check_a_rewrite_never_downgrades_the_scheme(policy);
    check_a_verdict_arrives_within_the_budget(policy);
}

// ---- the probes every adapter is asked about --------------------------------

/// A `GET` for a URL in the reserved `.invalid` TLD.
#[must_use]
pub fn probe_request() -> HttpRequest {
    HttpRequest::get(probe_url("/probe"))
}

/// The same target, asked with `HEAD`.
#[must_use]
pub fn head_probe_request() -> HttpRequest {
    HttpRequest::new(Method::Head, probe_url("/probe"))
}

/// A target no transport can be expected to reach.
#[must_use]
pub fn unreachable_probe_request() -> HttpRequest {
    HttpRequest::get(probe_url("/nothing-is-served-here?q=1"))
}

fn probe_url(target: &str) -> Url {
    Url::parse(&format!("{PROBE_ORIGIN}{target}"))
        .expect("the probe origin is a valid absolute URL")
}

fn probes() -> [HttpRequest; 3] {
    [
        probe_request(),
        head_probe_request(),
        unreachable_probe_request(),
    ]
}

// ---- transport rules --------------------------------------------------------

/// `PRD-009` item 2: a request always names an authority, because a [`Url`]
/// that does not cannot be built. The transport never has to defend against it.
fn check_a_request_cannot_exist_without_an_authority() {
    let hostless = Url::parse("http:///just-a-path");
    assert_eq!(
        hostless,
        Err(NetworkError::invalid_url(
            ProtocolPhase::Dns,
            UrlDefect::MissingHost
        )),
        "conformance: a URL with no authority must be a typed InvalidUrl"
    );
    let foreign_scheme = Url::parse("ftp://example.invalid/file");
    assert_eq!(
        foreign_scheme,
        Err(NetworkError::invalid_url(
            ProtocolPhase::Dns,
            UrlDefect::UnsupportedScheme
        )),
        "conformance: a scheme this engine does not speak must be a typed InvalidUrl"
    );
}

/// Totality: every probe produces a response or a typed error, and the error
/// names the phase it failed in.
fn check_every_probe_is_answered_typed(transport: &dyn HttpTransport) {
    for probe in probes() {
        let Err(error) = transport.execute(&probe) else {
            continue;
        };
        let rendered = format!("{error}");
        assert!(
            rendered.contains(error.phase_name()),
            "conformance: a NetworkError must render the phase it failed in, got {rendered:?}"
        );
    }
}

/// A response agrees with itself: the resolved media type is exactly what
/// re-parsing its own `Content-Type` gives.
fn check_an_answer_is_self_consistent(transport: &dyn HttpTransport) {
    for probe in probes() {
        let Ok(response) = transport.execute(&probe) else {
            continue;
        };
        assert_media_type_matches_headers(&response);
    }
}

fn assert_media_type_matches_headers(response: &HttpResponse) {
    let declared = response
        .headers()
        .text(&HeaderName::content_type())
        .and_then(|raw| MediaType::parse(raw).ok());
    assert_eq!(
        response.media_type(),
        declared.as_ref(),
        "conformance: the resolved media type must agree with the Content-Type header"
    );
}

/// Re-entrancy: `execute` takes `&self`, so the same request twice must give
/// the same answer. A connection pool may hand out a different socket; the
/// answer must not notice.
fn check_execute_is_re_entrant(transport: &dyn HttpTransport) {
    let probe = probe_request();
    let first = transport.execute(&probe);
    let second = transport.execute(&probe);
    assert_eq!(
        first.is_ok(),
        second.is_ok(),
        "conformance: execute must answer the same request the same way twice"
    );
    assert_answers_agree(&first, &second);
}

fn assert_answers_agree(
    first: &Result<HttpResponse, NetworkError>,
    second: &Result<HttpResponse, NetworkError>,
) {
    let (Err(first_error), Err(second_error)) = (first, second) else {
        return;
    };
    assert_eq!(
        first_error.phase(),
        second_error.phase(),
        "conformance: a repeated failure must fail in the same phase"
    );
}

/// Boundedness. A hostile peer must not be able to make `execute` never
/// return; a *true* hang is caught by the test harness, and this rule catches
/// the far commoner "eventually returned, after minutes".
fn check_an_answer_arrives_within_the_budget(transport: &dyn HttpTransport) {
    for probe in probes() {
        let started = Instant::now();
        drop(transport.execute(&probe));
        let elapsed = started.elapsed();
        assert!(
            elapsed <= ANSWER_BUDGET,
            "conformance: execute took {elapsed:?}, over the {ANSWER_BUDGET:?} budget"
        );
    }
}

/// A failure before any response byte arrived must not claim to be a body or
/// decode failure — the phase is the diagnosis, so it has to be true.
fn check_a_pre_response_failure_names_a_pre_response_phase(transport: &dyn HttpTransport) {
    let Err(error) = transport.execute(&unreachable_probe_request()) else {
        return;
    };
    assert!(
        matches!(
            error.phase(),
            ProtocolPhase::Dns
                | ProtocolPhase::Connect
                | ProtocolPhase::Handshake
                | ProtocolPhase::Header
                | ProtocolPhase::Redirect
        ),
        "conformance: a target that was never reached must not fail in the {} phase",
        error.phase_name()
    );
}

/// RFC 9112 §6.3: a response to `HEAD` carries no body, however it is framed.
fn check_a_head_answer_carries_no_body(transport: &dyn HttpTransport) {
    let Ok(response) = transport.execute(&head_probe_request()) else {
        return;
    };
    assert!(
        response.body().is_empty(),
        "conformance: a response to HEAD must carry no body"
    );
}

// ---- policy rules -----------------------------------------------------------

/// Totality: every probe gets a verdict or a typed error.
fn check_every_probe_gets_a_verdict(policy: &dyn RequestPolicy) {
    for probe in probes() {
        let Err(error) = policy.decide(&probe) else {
            continue;
        };
        assert_eq!(
            error.phase(),
            ProtocolPhase::Dns,
            "conformance: a policy failure happens before anything is on the wire"
        );
    }
}

/// Purity: `decide` takes `&self` and does no I/O, so it must be repeatable.
fn check_decide_is_pure(policy: &dyn RequestPolicy) {
    let probe = probe_request();
    let first = policy.decide(&probe);
    let second = policy.decide(&probe);
    assert_eq!(
        first, second,
        "conformance: decide must return the same verdict for the same request"
    );
}

/// A denial the user cannot be told the reason for is a bug report waiting to
/// happen.
fn check_a_denial_carries_a_reason(policy: &dyn RequestPolicy) {
    for probe in probes() {
        let Ok(PolicyVerdict::Deny { reason }) = policy.decide(&probe) else {
            continue;
        };
        assert!(
            !reason.trim().is_empty(),
            "conformance: a Deny verdict must carry a non-empty reason"
        );
    }
}

/// A rewrite may upgrade `http` to `https`; it may never do the reverse.
fn check_a_rewrite_never_downgrades_the_scheme(policy: &dyn RequestPolicy) {
    let secure = HttpRequest::get(
        Url::parse("https://probe.conformance.invalid/probe")
            .expect("the probe origin is a valid absolute URL"),
    );
    let Ok(PolicyVerdict::Rewrite(rewritten)) = policy.decide(&secure) else {
        return;
    };
    assert!(
        rewritten.url().scheme().is_secure(),
        "conformance: a rewrite must not downgrade an https request to http"
    );
}

/// A policy does no I/O, so a slow one is a policy bug.
fn check_a_verdict_arrives_within_the_budget(policy: &dyn RequestPolicy) {
    let started = Instant::now();
    drop(policy.decide(&probe_request()));
    let elapsed = started.elapsed();
    assert!(
        elapsed <= VERDICT_BUDGET,
        "conformance: decide took {elapsed:?}, over the {VERDICT_BUDGET:?} budget"
    );
}
