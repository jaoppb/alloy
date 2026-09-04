//! [`Scheme`] — the two URL schemes this engine speaks.
//!
//! A closed enum rather than a string, so "is this connection encrypted?" is a
//! `match` the compiler checks and never a `== "https"` comparison someone
//! forgets to lowercase (Object Calisthenics rule 3, `ADR-0010:129`).

use core::fmt;

use crate::domain::defect::UrlDefect;

/// The transport a [`Url`](crate::domain::url::Url) names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Scheme {
    /// Cleartext HTTP over TCP.
    Http,
    /// HTTP over TLS.
    Https,
}

impl Scheme {
    /// The lowercase scheme name, without the `://`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
        }
    }

    /// The port used when the authority names none.
    #[must_use]
    pub const fn default_port(self) -> u16 {
        match self {
            Self::Http => 80,
            Self::Https => 443,
        }
    }

    /// Whether this scheme puts TLS under the exchange.
    #[must_use]
    pub const fn is_secure(self) -> bool {
        matches!(self, Self::Https)
    }

    /// Parse a scheme name, case-insensitively.
    ///
    /// # Errors
    ///
    /// [`UrlDefect::UnsupportedScheme`] for anything but `http` and `https`.
    pub fn parse(raw: &str) -> Result<Self, UrlDefect> {
        let lowered = raw.to_ascii_lowercase();
        match lowered.as_str() {
            "http" => Ok(Self::Http),
            "https" => Ok(Self::Https),
            _ => Err(UrlDefect::UnsupportedScheme),
        }
    }
}

impl fmt::Display for Scheme {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
