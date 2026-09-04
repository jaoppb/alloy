//! [`HeaderName`], [`HeaderValue`] and [`HeaderMap`] — the first-class
//! collection of an HTTP field section (Object Calisthenics rule 4,
//! `ADR-0010:132`; the `dom::AttributeMap` pattern).
//!
//! Field names are case-insensitive on the wire (RFC 9110 §5.1), so
//! [`HeaderName`] lowercases on construction and the map is keyed by the
//! lowercased form — a lookup can never miss because a server capitalised
//! `Content-Type` differently. The backing [`BTreeMap`] makes both iteration
//! and serialisation deterministic, which the golden-image and byte-identity
//! tests downstream depend on.
//!
//! Repeated field lines are **combined with `", "`**, the rule RFC 9110 §5.3
//! gives for list-valued fields. `Set-Cookie` is the documented exception to
//! that rule in the spec; this engine parses no cookies in v0.5, so the
//! simplification is recorded here rather than worked around.

use core::fmt;
use std::collections::BTreeMap;

use crate::domain::defect::MalformedPart;
use crate::domain::error::NetworkError;
use crate::domain::phase::ProtocolPhase;

/// A validated, lowercased HTTP field name.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HeaderName(String);

impl HeaderName {
    /// Validate and lowercase a field name.
    ///
    /// # Errors
    ///
    /// [`NetworkError::Malformed`] with [`MalformedPart::HeaderName`] when the
    /// name is empty or carries a character outside RFC 9110's `token`.
    pub fn new(raw: &str) -> Result<Self, NetworkError> {
        if raw.is_empty() || !raw.chars().all(is_token_character) {
            return Err(NetworkError::malformed(
                ProtocolPhase::Header,
                MalformedPart::HeaderName,
            ));
        }
        Ok(Self(raw.to_ascii_lowercase()))
    }

    /// The lowercased field name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `host`
    #[must_use]
    pub fn host() -> Self {
        Self::known("host")
    }

    /// `content-length`
    #[must_use]
    pub fn content_length() -> Self {
        Self::known("content-length")
    }

    /// `content-type`
    #[must_use]
    pub fn content_type() -> Self {
        Self::known("content-type")
    }

    /// `content-encoding`
    #[must_use]
    pub fn content_encoding() -> Self {
        Self::known("content-encoding")
    }

    /// `transfer-encoding`
    #[must_use]
    pub fn transfer_encoding() -> Self {
        Self::known("transfer-encoding")
    }

    /// `connection`
    #[must_use]
    pub fn connection() -> Self {
        Self::known("connection")
    }

    /// `location`
    #[must_use]
    pub fn location() -> Self {
        Self::known("location")
    }

    /// `accept-encoding`
    #[must_use]
    pub fn accept_encoding() -> Self {
        Self::known("accept-encoding")
    }

    /// `user-agent`
    #[must_use]
    pub fn user_agent() -> Self {
        Self::known("user-agent")
    }

    /// `accept`
    #[must_use]
    pub fn accept() -> Self {
        Self::known("accept")
    }

    /// `authorization`
    #[must_use]
    pub fn authorization() -> Self {
        Self::known("authorization")
    }

    /// `cookie`
    #[must_use]
    pub fn cookie() -> Self {
        Self::known("cookie")
    }

    /// A name this crate spells itself. The literal is a lowercase token by
    /// inspection, so no validation — and therefore no `unwrap` — is needed.
    fn known(name: &'static str) -> Self {
        Self(String::from(name))
    }
}

/// RFC 9110 §5.6.2 `tchar`, minus the uppercase letters `new` folds away.
const fn is_token_character(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(
            character,
            '!' | '#'
                | '$'
                | '%'
                | '&'
                | '\''
                | '*'
                | '+'
                | '-'
                | '.'
                | '^'
                | '_'
                | '`'
                | '|'
                | '~'
        )
}

impl fmt::Display for HeaderName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A field value: the raw bytes, with CR, LF and NUL refused.
///
/// Bytes rather than `String` because field values are ISO-8859-1 on the wire
/// and a server may send anything; refusing CR and LF is the response-splitting
/// defence, and it is enforced in the constructor so no code path can bypass
/// it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HeaderValue(Vec<u8>);

impl HeaderValue {
    /// Validate raw field-value bytes, trimming optional leading and trailing
    /// whitespace (RFC 9110 §5.5 `OWS`).
    ///
    /// # Errors
    ///
    /// [`NetworkError::Malformed`] with [`MalformedPart::HeaderValue`] when a
    /// bare CR, LF or NUL is present.
    pub fn parse(raw: &[u8]) -> Result<Self, NetworkError> {
        if raw.iter().copied().any(is_forbidden_in_value) {
            return Err(NetworkError::malformed(
                ProtocolPhase::Header,
                MalformedPart::HeaderValue,
            ));
        }
        let trimmed: Vec<u8> = trim_optional_whitespace(raw).to_vec();
        Ok(Self(trimmed))
    }

    /// Validate a field value given as text.
    ///
    /// # Errors
    ///
    /// As [`HeaderValue::parse`].
    pub fn from_text(raw: &str) -> Result<Self, NetworkError> {
        Self::parse(raw.as_bytes())
    }

    /// The raw value bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// The value as text, when it happens to be UTF-8. A value that is not is
    /// not an error here — the caller that needs text decides what to do.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        core::str::from_utf8(&self.0).ok()
    }

    /// How many bytes the value occupies.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the value is the empty string.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Combine a repeated field line onto this one with `", "` (RFC 9110 §5.3).
    fn extend_field(&mut self, other: &Self) {
        self.0.extend_from_slice(b", ");
        self.0.extend_from_slice(&other.0);
    }
}

const fn is_forbidden_in_value(byte: u8) -> bool {
    matches!(byte, b'\r' | b'\n' | 0)
}

fn trim_optional_whitespace(raw: &[u8]) -> &[u8] {
    let start = raw
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(raw.len());
    let end = raw
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(start, |index| index.saturating_add(1));
    raw.get(start..end).unwrap_or_default()
}

impl fmt::Display for HeaderValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_str() {
            Some(text) => formatter.write_str(text),
            None => write!(formatter, "<{} non-UTF-8 bytes>", self.0.len()),
        }
    }
}

/// An HTTP field section: names to values, case-insensitive, deterministic.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeaderMap {
    entries: BTreeMap<HeaderName, HeaderValue>,
}

impl HeaderMap {
    /// An empty field section.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Insert `name`, replacing any value it already had.
    pub fn set(&mut self, name: HeaderName, value: HeaderValue) {
        self.entries.insert(name, value);
    }

    /// Insert `name`, or combine onto its existing value with `", "` when the
    /// peer sent the field line twice (RFC 9110 §5.3).
    pub fn append(&mut self, name: HeaderName, value: HeaderValue) {
        let Some(existing) = self.entries.get_mut(&name) else {
            self.entries.insert(name, value);
            return;
        };
        existing.extend_field(&value);
    }

    /// The value of `name`, if the section carries it.
    #[must_use]
    pub fn get(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.entries.get(name)
    }

    /// The value of `name` as text, when it is present and UTF-8.
    #[must_use]
    pub fn text(&self, name: &HeaderName) -> Option<&str> {
        self.entries.get(name).and_then(HeaderValue::as_str)
    }

    /// Whether the section carries `name`.
    #[must_use]
    pub fn contains(&self, name: &HeaderName) -> bool {
        self.entries.contains_key(name)
    }

    /// Remove `name` if present; answers whether it was there.
    pub fn remove(&mut self, name: &HeaderName) -> bool {
        self.entries.remove(name).is_some()
    }

    /// Every field, in deterministic name order.
    pub fn iter(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> + '_ {
        self.entries.iter()
    }

    /// How many distinct field names the section carries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the section is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
