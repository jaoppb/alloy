//! [`Body`] — the first-class collection of a message's payload bytes.
//!
//! A newtype rather than a bare `Vec<u8>` (Object Calisthenics rules 3 and 4,
//! `ADR-0010:129-132`), so an aggregate never exposes a mutable buffer and a
//! signature says *body* rather than *some bytes*.

use core::fmt;

/// A request or response payload.
///
/// On a response this is the **decoded** payload: any `Content-Encoding` has
/// been undone and, for a textual media type, the bytes have been transcoded to
/// UTF-8. `core/html` therefore receives UTF-8 and never has to guess.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Body {
    bytes: Vec<u8>,
}

impl Body {
    /// A body with no bytes.
    #[must_use]
    pub const fn empty() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Take ownership of a byte buffer.
    #[must_use]
    pub const fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// Copy a byte slice.
    #[must_use]
    pub fn from_slice(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
        }
    }

    /// Copy text, encoded as UTF-8.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        Self {
            bytes: text.as_bytes().to_vec(),
        }
    }

    /// The payload bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The payload as text, when it is valid UTF-8.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        core::str::from_utf8(&self.bytes).ok()
    }

    /// How many bytes the payload occupies.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Whether the payload is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl fmt::Display for Body {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "<{} bytes>", self.bytes.len())
    }
}
