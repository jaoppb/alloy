//! [`StatusCode`] — a validated three-digit HTTP response status.

use core::fmt;

use crate::domain::defect::MalformedPart;
use crate::domain::error::NetworkError;
use crate::domain::phase::ProtocolPhase;

/// The lowest status code RFC 9110 permits.
const LOWEST: u16 = 100;
/// The highest status code RFC 9110 permits.
const HIGHEST: u16 = 599;

/// An HTTP response status code, guaranteed to be in `100..=599`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StatusCode(u16);

impl StatusCode {
    /// `200 OK`, the status a fixture reaches for most often.
    pub const OK: Self = Self(200);
    /// `301 Moved Permanently`.
    pub const MOVED_PERMANENTLY: Self = Self(301);
    /// `302 Found`.
    pub const FOUND: Self = Self(302);
    /// `303 See Other`.
    pub const SEE_OTHER: Self = Self(303);
    /// `307 Temporary Redirect`.
    pub const TEMPORARY_REDIRECT: Self = Self(307);
    /// `308 Permanent Redirect`.
    pub const PERMANENT_REDIRECT: Self = Self(308);
    /// `304 Not Modified` — a redirect-class code that carries no body and is
    /// not a redirect to follow.
    pub const NOT_MODIFIED: Self = Self(304);
    /// `404 Not Found`.
    pub const NOT_FOUND: Self = Self(404);

    /// Wrap a status number.
    ///
    /// # Errors
    ///
    /// [`NetworkError::Malformed`] with [`MalformedPart::StatusLineCode`] when
    /// the number is outside `100..=599`.
    pub const fn new(code: u16) -> Result<Self, NetworkError> {
        if code < LOWEST || code > HIGHEST {
            return Err(NetworkError::malformed(
                ProtocolPhase::Header,
                MalformedPart::StatusLineCode,
            ));
        }
        Ok(Self(code))
    }

    /// The status number.
    #[must_use]
    pub const fn code(self) -> u16 {
        self.0
    }

    /// `1xx`
    #[must_use]
    pub const fn is_informational(self) -> bool {
        self.0 < 200
    }

    /// `2xx`
    #[must_use]
    pub const fn is_success(self) -> bool {
        self.0 >= 200 && self.0 < 300
    }

    /// `3xx`
    #[must_use]
    pub const fn is_redirect(self) -> bool {
        self.0 >= 300 && self.0 < 400
    }

    /// `4xx`
    #[must_use]
    pub const fn is_client_error(self) -> bool {
        self.0 >= 400 && self.0 < 500
    }

    /// `5xx`
    #[must_use]
    pub const fn is_server_error(self) -> bool {
        self.0 >= 500
    }

    /// Whether a response with this status is defined to carry no body at all,
    /// however it is framed (RFC 9112 §6.3: `1xx`, `204`, `304`).
    #[must_use]
    pub const fn forbids_body(self) -> bool {
        self.is_informational() || self.0 == 204 || self.0 == 304
    }

    /// Whether this is a redirect a client should follow — a `3xx` that names
    /// a `Location`, which `304 Not Modified` deliberately is not.
    #[must_use]
    pub const fn is_followable_redirect(self) -> bool {
        self.is_redirect() && self.0 != 304 && self.0 != 300 && self.0 != 305 && self.0 != 306
    }

    /// Whether a redirect with this status rewrites a non-idempotent method to
    /// `GET` (RFC 9110 §15.4.2-4: `301`, `302`, `303` do; `307`, `308` do not).
    #[must_use]
    pub const fn rewrites_method(self) -> bool {
        self.0 == 301 || self.0 == 302 || self.0 == 303
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}
