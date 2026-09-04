//! The in-repo adapters `ADR-0011` item 6 asks every port for:
//! [`MockTransport`] and [`AllowAllPolicy`].
//!
//! Neither is feature-gated: they are what the `no-transport`
//! (`--no-default-features`) build has instead of a socket, so the ports still
//! have a working implementation to point at when no crypto is linked.

use std::collections::BTreeMap;
use std::io::BufReader;

use crate::application::ports::{HttpTransport, PolicyVerdict, RequestPolicy};
use crate::domain::body::Body;
use crate::domain::error::NetworkError;
use crate::domain::method::Method;
use crate::domain::request::HttpRequest;
use crate::domain::response::HttpResponse;
use crate::domain::url::Url;
use crate::infrastructure::deadline::Deadline;
use crate::infrastructure::deadline::PhaseTimeouts;
use crate::infrastructure::http1::exchange::read_response;
use crate::infrastructure::limits::WireLimits;

/// A transport that answers from a fixture map instead of a network.
///
/// Deterministic by construction — the same request always gets the same
/// answer, which is what lets it pass the re-entrancy rule of
/// [`run_transport_suite`](crate::application::conformance::run_transport_suite).
/// A target it does not serve is
/// [`NetworkError::Unreachable`](crate::domain::error::NetworkError::Unreachable),
/// the same class of failure a real socket would report, in the same phase.
#[derive(Clone, Debug, Default)]
pub struct MockTransport {
    fixtures: BTreeMap<Url, HttpResponse>,
}

impl MockTransport {
    /// A transport that serves nothing.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fixtures: BTreeMap::new(),
        }
    }

    /// The same transport, also serving `response` for `url`.
    #[must_use]
    pub fn with_response(mut self, url: Url, response: HttpResponse) -> Self {
        self.fixtures.insert(url, response);
        self
    }

    /// The same transport, also serving the response encoded in `wire` — raw
    /// HTTP/1.1 bytes, as `core/network/tests/fixtures/` stores them.
    ///
    /// # Errors
    ///
    /// Whatever [`read_response`] raises for that fixture: this is the seam a
    /// hostile-fixture test drives, so a malformed fixture must fail typed
    /// here rather than be silently accepted.
    pub fn with_wire_response(mut self, url: Url, wire: &[u8]) -> Result<Self, NetworkError> {
        let response = Self::parse_wire(wire)?;
        self.fixtures.insert(url, response);
        Ok(self)
    }

    /// Parse raw HTTP/1.1 response bytes into a response.
    ///
    /// # Errors
    ///
    /// As [`read_response`].
    pub fn parse_wire(wire: &[u8]) -> Result<HttpResponse, NetworkError> {
        let mut reader = BufReader::new(wire);
        let deadline = Deadline::starting_now(PhaseTimeouts::DEFAULT.total());
        read_response(&mut reader, Method::Get, WireLimits::DEFAULT, &deadline)
    }

    /// How many targets this transport serves.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fixtures.len()
    }

    /// Whether this transport serves nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fixtures.is_empty()
    }
}

impl HttpTransport for MockTransport {
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, NetworkError> {
        let url = request.url();
        let response = self.fixtures.get(url).cloned().ok_or_else(|| {
            NetworkError::unreachable(url.authority().to_text(), "no fixture serves this target")
        })?;
        if request.method().allows_response_body() {
            return Ok(response);
        }
        Ok(response.with_body(Body::empty()))
    }
}

/// A policy that permits every request.
///
/// **Not a safe default for untrusted content.** Deciding what a page may
/// fetch — same-origin rules, private-address (SSRF) refusal, scheme upgrade —
/// is exactly what [`RequestPolicy`] exists for, and Phase M installs a real
/// one written in `.rhai`. This adapter exists so the port has a trivial
/// reference implementation and the conformance suite has something to run.
#[derive(Clone, Copy, Debug, Default)]
pub struct AllowAllPolicy;

impl AllowAllPolicy {
    /// The one and only instance.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl RequestPolicy for AllowAllPolicy {
    fn decide(&self, _request: &HttpRequest) -> Result<PolicyVerdict, NetworkError> {
        Ok(PolicyVerdict::Allow)
    }
}
