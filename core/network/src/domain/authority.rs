//! [`Host`], [`Port`] and [`Authority`] — the "where" half of a URL.
//!
//! Validated newtypes, never a naked `String` or `u16` (Object Calisthenics
//! rule 3, `ADR-0010:129`), following the `dom::AttributeName` pattern:
//! validate and normalise once, in the constructor.

use core::fmt;

use crate::domain::defect::UrlDefect;
use crate::domain::scheme::Scheme;

/// A validated host: a registered name or an IP literal, lowercased.
///
/// Non-empty, ASCII only, no whitespace, no control characters, and none of
/// the delimiters that would let a crafted host smuggle a second request into
/// the `Host:` header.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Host(String);

impl Host {
    /// Validate and lowercase a host.
    ///
    /// # Errors
    ///
    /// [`UrlDefect::MissingHost`] when empty, [`UrlDefect::MalformedHost`]
    /// when a character is not permitted.
    pub fn new(raw: &str) -> Result<Self, UrlDefect> {
        if raw.is_empty() {
            return Err(UrlDefect::MissingHost);
        }
        if raw.chars().any(is_forbidden_in_host) {
            return Err(UrlDefect::MalformedHost);
        }
        Ok(Self(raw.to_ascii_lowercase()))
    }

    /// The lowercased host text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// `[` and `]` are permitted for IPv6 literals; `:` is not — the port is split
/// off before a `Host` is ever built.
const fn is_forbidden_in_host(character: char) -> bool {
    if matches!(character, '[' | ']') {
        return false;
    }
    !character.is_ascii()
        || character.is_ascii_control()
        || character.is_ascii_whitespace()
        || matches!(
            character,
            ':' | '/' | '?' | '#' | '@' | '\\' | '"' | '\'' | '<' | '>' | '{' | '}' | '|' | '^'
        )
}

impl fmt::Display for Host {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A validated TCP port: `1..=65535`. Port zero is not a destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Port(u16);

impl Port {
    /// Wrap a port number.
    ///
    /// # Errors
    ///
    /// [`UrlDefect::MalformedPort`] for zero.
    pub const fn new(number: u16) -> Result<Self, UrlDefect> {
        if number == 0 {
            return Err(UrlDefect::MalformedPort);
        }
        Ok(Self(number))
    }

    /// Parse the digits after `:` in an authority.
    ///
    /// # Errors
    ///
    /// [`UrlDefect::MalformedPort`] when absent, non-numeric, zero, or above
    /// `65535`.
    pub fn parse(raw: &str) -> Result<Self, UrlDefect> {
        let number = raw.parse::<u16>().map_err(|_| UrlDefect::MalformedPort)?;
        Self::new(number)
    }

    /// The port a scheme uses when the authority names none.
    #[must_use]
    pub const fn default_for(scheme: Scheme) -> Self {
        Self(scheme.default_port())
    }

    /// The port number.
    #[must_use]
    pub const fn number(self) -> u16 {
        self.0
    }
}

impl fmt::Display for Port {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// A host and a port together — the key a connection pool is bucketed by and
/// the value the `Host:` header carries.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Authority {
    host: Host,
    port: Port,
}

impl Authority {
    /// Pair a host with a port.
    #[must_use]
    pub const fn new(host: Host, port: Port) -> Self {
        Self { host, port }
    }

    /// The host half.
    #[must_use]
    pub const fn host(&self) -> &Host {
        &self.host
    }

    /// The port half.
    #[must_use]
    pub const fn port(&self) -> Port {
        self.port
    }

    /// `host:port`, always explicit — what a pool key and a log line want.
    #[must_use]
    pub fn to_text(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// The `Host:` header value: the port is elided when it is `scheme`'s
    /// default, as RFC 9110 §7.2 expects.
    #[must_use]
    pub fn to_header_text(&self, scheme: Scheme) -> String {
        if self.port.number() == scheme.default_port() {
            return self.host.as_str().to_owned();
        }
        self.to_text()
    }
}

impl fmt::Display for Authority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_text())
    }
}
