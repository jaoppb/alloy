//! [`Method`] — the HTTP request methods this engine can put on a request
//! line.

use core::fmt;

/// An HTTP request method.
///
/// `#[non_exhaustive]`: a later version may speak more of them, and a consumer
/// must not assume this list is closed (`ADR-0011` item 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Method {
    /// Retrieve a representation.
    Get,
    /// Retrieve the head of a representation, without its body.
    Head,
    /// Submit a representation to the target.
    Post,
    /// Replace the target with a representation.
    Put,
    /// Remove the target.
    Delete,
    /// Apply a partial modification.
    Patch,
    /// Ask what the target supports.
    Options,
    /// Echo the request back.
    Trace,
    /// Retrieve a representation selected by a request body, when the target
    /// is too complex for a query string (IETF `HTTP QUERY Method`).
    Query,
}

impl Method {
    /// The uppercase token that goes on the request line.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
            Self::Options => "OPTIONS",
            Self::Trace => "TRACE",
            Self::Query => "QUERY",
        }
    }

    /// Whether a response to this method may carry a body at all
    /// (RFC 9112 §6.3: a response to `HEAD` never does, however it is framed).
    #[must_use]
    pub const fn allows_response_body(self) -> bool {
        !matches!(self, Self::Head)
    }

    /// Whether this method is one a 301/302/303 redirect rewrites to `GET`
    /// (RFC 9110 §15.4). `QUERY` is safe and idempotent like `GET`/`HEAD`, so
    /// it is exempt too.
    #[must_use]
    pub const fn is_rewritten_on_redirect(self) -> bool {
        !matches!(self, Self::Get | Self::Head | Self::Query)
    }
}

impl fmt::Display for Method {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
