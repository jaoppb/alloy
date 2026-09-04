//! [`ProtocolPhase`] — *where* on the wire a failure happened.
//!
//! This port's location metadata (`ADR-0011:93-95`), the analogue of
//! `engine`'s `SourceLocation` and `css`'s `CssStage`. Every [`NetworkError`]
//! variant carries one, so a caller can tell "the name never resolved" from
//! "the certificate was rejected" from "the body was framed wrong" without
//! matching on prose.
//!
//! [`NetworkError`]: crate::domain::error::NetworkError

use core::fmt;

/// The phase of an HTTP exchange that raised an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ProtocolPhase {
    /// Turning an authority into socket addresses — and, before that, turning
    /// text into a [`Url`](crate::domain::url::Url) at all.
    Dns,
    /// Opening the TCP connection.
    Connect,
    /// The TLS handshake.
    Handshake,
    /// Reading or writing the status line and the header block.
    Header,
    /// Reading the message body, including de-chunking.
    Body,
    /// Following a `Location` — the limit and the cycle check.
    Redirect,
    /// Content-coding (`gzip` / `deflate`) or charset decoding of the body.
    Decode,
}

impl ProtocolPhase {
    /// The lowercase wire name of this phase, stable for diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Dns => "dns",
            Self::Connect => "connect",
            Self::Handshake => "handshake",
            Self::Header => "header",
            Self::Body => "body",
            Self::Redirect => "redirect",
            Self::Decode => "decode",
        }
    }
}

impl fmt::Display for ProtocolPhase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}
