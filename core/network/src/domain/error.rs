//! [`NetworkError`] — the **one** typed error for this port (`ADR-0011` item 4).
//!
//! `thiserror`, not a hand-written `Display`: the manual carve-out of
//! `ADR-0015` applies only to `core/engine`; this crate follows `core/dom` and
//! `core/css` (correction at the top of the v0.5 plan).
//!
//! Two rules hold for every variant:
//!
//! 1. It carries a [`ProtocolPhase`] — the location metadata of
//!    `ADR-0011:93-95`, so a caller can tell *where* on the wire it broke.
//! 2. Its reason is a **typed defect enum** wherever the cause is one of a
//!    closed set. Only genuinely open-ended text (an OS error string, a policy
//!    author's message) is a `String`.

use crate::domain::defect::{
    DecodeDefect, FramingDefect, MalformedPart, RedirectDefect, UrlDefect, WireLimit,
};
use crate::domain::phase::ProtocolPhase;

/// A failure raised while deciding on, sending, or reading back an HTTP
/// exchange.
///
/// `Eq` as well as `PartialEq` (every field is `Eq`-capable and the `nursery`
/// lint requires it) — matching `core/dom`'s `DomError` and `core/css`'s
/// `CssError`.
#[derive(thiserror::Error, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NetworkError {
    /// A string could not become a [`Url`](crate::domain::url::Url).
    #[error("{phase} phase: invalid URL — {defect}")]
    InvalidUrl {
        /// Where this was noticed.
        phase: ProtocolPhase,
        /// What was wrong with the text.
        defect: UrlDefect,
    },

    /// An authority did not resolve to any socket address.
    #[error("{phase} phase: cannot resolve {host} — {reason}")]
    Unresolved {
        /// Where this was noticed.
        phase: ProtocolPhase,
        /// The authority that did not resolve.
        host: String,
        /// The resolver's own words.
        reason: String,
    },

    /// No connection could be opened to the authority.
    #[error("{phase} phase: cannot reach {authority} — {reason}")]
    Unreachable {
        /// Where this was noticed.
        phase: ProtocolPhase,
        /// The `host:port` that refused, or that nothing was fixtured for.
        authority: String,
        /// The socket's own words.
        reason: String,
    },

    /// The TLS handshake did not complete.
    #[error("{phase} phase: TLS handshake with {host} failed — {reason}")]
    HandshakeRejected {
        /// Where this was noticed.
        phase: ProtocolPhase,
        /// The host whose certificate chain or negotiation was refused.
        host: String,
        /// The TLS stack's own words.
        reason: String,
    },

    /// The peer sent a head this parser cannot accept.
    #[error("{phase} phase: malformed message — {part}")]
    Malformed {
        /// Where this was noticed.
        phase: ProtocolPhase,
        /// Which part of the head was wrong.
        part: MalformedPart,
    },

    /// The peer framed the body wrong, or stopped short of finishing it.
    #[error("{phase} phase: bad body framing — {defect}")]
    Framing {
        /// Where this was noticed.
        phase: ProtocolPhase,
        /// How the framing was wrong.
        defect: FramingDefect,
    },

    /// A hostile or merely enormous peer pushed past a ceiling.
    #[error("{phase} phase: {limit} (observed {observed} bytes)")]
    LimitExceeded {
        /// Where this was noticed.
        phase: ProtocolPhase,
        /// Which ceiling was hit.
        limit: WireLimit,
        /// How far past it we had counted when we stopped.
        observed: usize,
    },

    /// A redirect chain was abandoned.
    #[error("{phase} phase: redirect abandoned — {defect}")]
    Redirect {
        /// Where this was noticed.
        phase: ProtocolPhase,
        /// Why it was abandoned.
        defect: RedirectDefect,
    },

    /// A body could not be decompressed or transcoded.
    #[error("{phase} phase: cannot decode body — {defect}")]
    Decode {
        /// Where this was noticed.
        phase: ProtocolPhase,
        /// Why not.
        defect: DecodeDefect,
    },

    /// A phase outlived its budget. The exchange is abandoned, never retried
    /// silently, and never left hanging (`PRD-009` hostile-server rule).
    #[error("{phase} phase: timed out after {elapsed_millis} ms")]
    Timeout {
        /// Which phase ran out of time.
        phase: ProtocolPhase,
        /// How long it had run.
        elapsed_millis: u64,
    },

    /// An operating-system I/O error with no more specific home.
    #[error("{phase} phase: transport error — {reason}")]
    Transport {
        /// Where this was noticed.
        phase: ProtocolPhase,
        /// The OS's own words.
        reason: String,
    },

    /// A [`RequestPolicy`](crate::application::ports::RequestPolicy) refused
    /// the request. No socket was ever opened.
    #[error("{phase} phase: request denied by policy — {reason}")]
    PolicyDenied {
        /// Always [`ProtocolPhase::Dns`] — policy runs before anything else.
        phase: ProtocolPhase,
        /// The policy author's own words.
        reason: String,
    },
}

impl NetworkError {
    /// A string that is not a usable URL.
    #[must_use]
    pub const fn invalid_url(phase: ProtocolPhase, defect: UrlDefect) -> Self {
        Self::InvalidUrl { phase, defect }
    }

    /// An authority that did not resolve.
    #[must_use]
    pub fn unresolved(host: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Unresolved {
            phase: ProtocolPhase::Dns,
            host: host.into(),
            reason: reason.into(),
        }
    }

    /// An authority that resolved but could not be connected to — also the
    /// error a fixture-backed transport raises for a target it does not serve.
    #[must_use]
    pub fn unreachable(authority: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Unreachable {
            phase: ProtocolPhase::Connect,
            authority: authority.into(),
            reason: reason.into(),
        }
    }

    /// A TLS handshake that did not complete.
    #[must_use]
    pub fn handshake_rejected(host: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::HandshakeRejected {
            phase: ProtocolPhase::Handshake,
            host: host.into(),
            reason: reason.into(),
        }
    }

    /// A head this parser cannot accept.
    #[must_use]
    pub const fn malformed(phase: ProtocolPhase, part: MalformedPart) -> Self {
        Self::Malformed { phase, part }
    }

    /// A body framed wrong.
    #[must_use]
    pub const fn framing(defect: FramingDefect) -> Self {
        Self::Framing {
            phase: ProtocolPhase::Body,
            defect,
        }
    }

    /// A ceiling pushed past.
    #[must_use]
    pub const fn limit_exceeded(phase: ProtocolPhase, limit: WireLimit, observed: usize) -> Self {
        Self::LimitExceeded {
            phase,
            limit,
            observed,
        }
    }

    /// A redirect chain abandoned.
    #[must_use]
    pub const fn redirect(defect: RedirectDefect) -> Self {
        Self::Redirect {
            phase: ProtocolPhase::Redirect,
            defect,
        }
    }

    /// A body that could not be decompressed or transcoded.
    #[must_use]
    pub const fn decode(defect: DecodeDefect) -> Self {
        Self::Decode {
            phase: ProtocolPhase::Decode,
            defect,
        }
    }

    /// A phase that outlived its budget.
    #[must_use]
    pub const fn timeout(phase: ProtocolPhase, elapsed_millis: u64) -> Self {
        Self::Timeout {
            phase,
            elapsed_millis,
        }
    }

    /// An OS I/O error with no more specific home.
    #[must_use]
    pub fn transport(phase: ProtocolPhase, reason: impl Into<String>) -> Self {
        Self::Transport {
            phase,
            reason: reason.into(),
        }
    }

    /// A request a [`RequestPolicy`](crate::application::ports::RequestPolicy)
    /// refused.
    #[must_use]
    pub fn policy_denied(reason: impl Into<String>) -> Self {
        Self::PolicyDenied {
            phase: ProtocolPhase::Dns,
            reason: reason.into(),
        }
    }

    /// The phase this error was raised in.
    #[must_use]
    pub const fn phase(&self) -> ProtocolPhase {
        match self {
            Self::InvalidUrl { phase, .. }
            | Self::Unresolved { phase, .. }
            | Self::Unreachable { phase, .. }
            | Self::HandshakeRejected { phase, .. }
            | Self::Malformed { phase, .. }
            | Self::Framing { phase, .. }
            | Self::LimitExceeded { phase, .. }
            | Self::Redirect { phase, .. }
            | Self::Decode { phase, .. }
            | Self::Timeout { phase, .. }
            | Self::Transport { phase, .. }
            | Self::PolicyDenied { phase, .. } => *phase,
        }
    }

    /// The same error re-labelled with the phase the caller was actually in.
    ///
    /// A redirect hop that fails to parse its `Location` is a *redirect*
    /// failure even though [`Url::parse`](crate::domain::url::Url::parse)
    /// reports the DNS phase by default.
    #[must_use]
    pub const fn in_phase(mut self, phase: ProtocolPhase) -> Self {
        *Self::phase_slot(&mut self) = phase;
        self
    }

    /// The phase name alone — what `tracing` records as a field.
    #[must_use]
    pub const fn phase_name(&self) -> &'static str {
        self.phase().name()
    }

    const fn phase_slot(&mut self) -> &mut ProtocolPhase {
        match self {
            Self::InvalidUrl { phase, .. }
            | Self::Unresolved { phase, .. }
            | Self::Unreachable { phase, .. }
            | Self::HandshakeRejected { phase, .. }
            | Self::Malformed { phase, .. }
            | Self::Framing { phase, .. }
            | Self::LimitExceeded { phase, .. }
            | Self::Redirect { phase, .. }
            | Self::Decode { phase, .. }
            | Self::Timeout { phase, .. }
            | Self::Transport { phase, .. }
            | Self::PolicyDenied { phase, .. } => phase,
        }
    }
}
