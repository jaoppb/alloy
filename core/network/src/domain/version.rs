//! [`HttpVersion`] — the version token at the front of a status line.

use core::fmt;

/// The HTTP version this engine accepts on a status line.
///
/// `#[non_exhaustive]`: a later version may speak HTTP/2, and a consumer must
/// not assume this list is closed (`ADR-0011` item 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HttpVersion {
    /// `HTTP/1.0`.
    Http10,
    /// `HTTP/1.1`.
    Http11,
}

impl HttpVersion {
    /// The exact token this engine reads on the wire.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Http10 => "HTTP/1.0",
            Self::Http11 => "HTTP/1.1",
        }
    }

    /// Parse the version token of a status line.
    ///
    /// `None` for anything other than `HTTP/1.0` or `HTTP/1.1` — this engine
    /// speaks no other version.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "HTTP/1.1" => Some(Self::Http11),
            "HTTP/1.0" => Some(Self::Http10),
            _ => None,
        }
    }
}

impl fmt::Display for HttpVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
