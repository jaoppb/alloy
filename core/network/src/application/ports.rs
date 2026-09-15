//! The replaceable ports of `PRD-009`: [`HttpTransport`] (mechanism) and
//! [`RequestPolicy`] (policy).
//!
//! Both are object-safe and `Send + Sync`: every method speaks only this
//! crate's own types, so `&dyn` handles work directly and `ADR-0011` item 2 is
//! satisfied without a companion trait — the same shape as `graphics`'s
//! `RenderBackend` and `css`'s `CascadeResolver`. **No `rustls`, `TcpStream` or
//! `webpki` type appears in any signature here**; those names exist only inside
//! [`infrastructure`](crate::infrastructure).
//!
//! ## Why the traits are synchronous (`ADR-0019`)
//!
//! The window event loop owns the main thread and there is no async runtime in
//! this workspace. A consumer runs [`HttpTransport::execute`] on a worker of a
//! `std::thread` pool and receives the [`HttpResponse`] back over
//! `std::sync::mpsc` as one more event the loop drains
//! (`docs/adr/0019-single-event-loop-owns-the-main-thread.md`). That is a
//! contract item, not a convention: an implementation may block, and a caller
//! must assume it will.
//!
//! ## Policy runs before mechanism
//!
//! [`RequestPolicy::decide`] is consulted **before** any socket is opened, so a
//! [`PolicyVerdict::Deny`] costs no connection and leaks no DNS query. This is
//! the seam the scriptable network policy of Phase M plugs into.

use crate::domain::error::NetworkError;
use crate::domain::request::HttpRequest;
use crate::domain::response::HttpResponse;

/// Performs one HTTP exchange.
///
/// Implementations must be **total**: a hostile or broken peer produces a typed
/// [`NetworkError`] carrying the [`ProtocolPhase`] it failed in — never a
/// panic, and never an unbounded wait. Every socket read and write is under a
/// timeout.
///
/// [`ProtocolPhase`]: crate::domain::phase::ProtocolPhase
pub trait HttpTransport: Send + Sync {
    /// Send `request` and read the response back, following redirects and
    /// decoding the body.
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, NetworkError>;
}

/// Decides whether a request may be made at all, and in what form.
///
/// Pure and cheap: it opens nothing and blocks on nothing. A policy that needs
/// I/O to decide belongs on the other side of the seam.
pub trait RequestPolicy: Send + Sync {
    /// Judge `request`.
    fn decide(&self, request: &HttpRequest) -> Result<PolicyVerdict, NetworkError>;
}

/// What a [`RequestPolicy`] decided.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PolicyVerdict {
    /// Send the request as it stands.
    Allow,
    /// Send this request instead — an upgrade to `https`, a proxy rewrite, a
    /// stripped header.
    Rewrite(HttpRequest),
    /// Send nothing. The caller turns this into
    /// [`NetworkError::PolicyDenied`].
    Deny {
        /// Why, in the policy author's words. Shown to the user and logged.
        reason: String,
    },
}

impl PolicyVerdict {
    /// A denial with a reason.
    #[must_use]
    pub fn deny(reason: impl Into<String>) -> Self {
        Self::Deny {
            reason: reason.into(),
        }
    }

    /// The request this verdict says to send, if it says to send one.
    #[must_use]
    pub const fn request_to_send<'verdict>(
        &'verdict self,
        original: &'verdict HttpRequest,
    ) -> Option<&'verdict HttpRequest> {
        match self {
            Self::Allow => Some(original),
            Self::Rewrite(rewritten) => Some(rewritten),
            Self::Deny { .. } => None,
        }
    }
}
