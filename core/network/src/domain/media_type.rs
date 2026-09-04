//! [`MediaType`] and [`Charset`] — what a body *is*, resolved once from
//! `Content-Type`.
//!
//! [`Charset`] is domain, not infrastructure: it is the *label*, a closed set
//! of encodings this engine implements. The 256-entry decoding table that turns
//! bytes into text lives in
//! [`infrastructure::charset`](crate::infrastructure::charset).

use core::fmt;

use crate::domain::defect::DecodeDefect;
use crate::domain::error::NetworkError;

/// A text encoding this engine can decode.
///
/// Deliberately two: UTF-8, and the windows-1252 fallback the HTML standard
/// prescribes for unlabelled legacy content. Anything else is a typed error,
/// never silent mojibake (v0.5 plan, "Fase C1").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Charset {
    /// UTF-8. Invalid sequences decode to `U+FFFD`.
    Utf8,
    /// windows-1252, the WHATWG superset of ISO-8859-1.
    Windows1252,
}

impl Charset {
    /// The canonical label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Utf8 => "utf-8",
            Self::Windows1252 => "windows-1252",
        }
    }

    /// Resolve a charset label, case-insensitively, through the aliases the
    /// WHATWG Encoding Standard lists for these two encodings.
    ///
    /// # Errors
    ///
    /// [`NetworkError::Decode`] with [`DecodeDefect::UnsupportedCharset`] for
    /// any other label — visible, rather than decoded as garbage.
    pub fn from_label(raw: &str) -> Result<Self, NetworkError> {
        let lowered = raw.trim().trim_matches('"').to_ascii_lowercase();
        match lowered.as_str() {
            "utf-8" | "utf8" | "unicode-1-1-utf-8" | "us-ascii" | "ascii" => Ok(Self::Utf8),
            "windows-1252" | "cp1252" | "iso-8859-1" | "iso8859-1" | "latin1" | "l1"
            | "iso_8859-1" => Ok(Self::Windows1252),
            _ => Err(NetworkError::decode(DecodeDefect::UnsupportedCharset)),
        }
    }
}

impl fmt::Display for Charset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// A parsed `Content-Type`: a type, a subtype, and the `charset` parameter if
/// the sender gave one.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MediaType {
    type_name: String,
    subtype: String,
    charset: Option<Charset>,
}

impl MediaType {
    /// Parse a `Content-Type` field value.
    ///
    /// Parameters other than `charset` are ignored; an *unsupported* `charset`
    /// is an error, because silently ignoring it is how a page renders as
    /// mojibake.
    ///
    /// # Errors
    ///
    /// [`NetworkError::Decode`] with [`DecodeDefect::MalformedMediaType`] when
    /// there is no `type/subtype`, or with
    /// [`DecodeDefect::UnsupportedCharset`] when the `charset` parameter names
    /// an encoding this engine does not implement.
    pub fn parse(raw: &str) -> Result<Self, NetworkError> {
        let mut parts = raw.split(';');
        let essence = parts.next().unwrap_or_default().trim();
        let (type_name, subtype) = essence
            .split_once('/')
            .ok_or_else(|| NetworkError::decode(DecodeDefect::MalformedMediaType))?;
        if type_name.is_empty() || subtype.is_empty() {
            return Err(NetworkError::decode(DecodeDefect::MalformedMediaType));
        }
        let charset = Self::charset_parameter(parts)?;
        Ok(Self {
            type_name: type_name.trim().to_ascii_lowercase(),
            subtype: subtype.trim().to_ascii_lowercase(),
            charset,
        })
    }

    fn charset_parameter<'a>(
        parameters: impl Iterator<Item = &'a str>,
    ) -> Result<Option<Charset>, NetworkError> {
        for parameter in parameters {
            let Some((key, value)) = parameter.split_once('=') else {
                continue;
            };
            if !key.trim().eq_ignore_ascii_case("charset") {
                continue;
            }
            return Charset::from_label(value).map(Some);
        }
        Ok(None)
    }

    /// Build a media type from already-normalised parts.
    #[must_use]
    pub fn new(type_name: &str, subtype: &str, charset: Option<Charset>) -> Self {
        Self {
            type_name: type_name.to_ascii_lowercase(),
            subtype: subtype.to_ascii_lowercase(),
            charset,
        }
    }

    /// The type half, lowercased — `text` in `text/html`.
    #[must_use]
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// The subtype half, lowercased — `html` in `text/html`.
    #[must_use]
    pub fn subtype(&self) -> &str {
        &self.subtype
    }

    /// The `charset` parameter, if the sender gave a supported one.
    #[must_use]
    pub const fn charset(&self) -> Option<Charset> {
        self.charset
    }

    /// `type/subtype`, without parameters.
    #[must_use]
    pub fn essence(&self) -> String {
        format!("{}/{}", self.type_name, self.subtype)
    }

    /// Whether a body of this media type is text a charset applies to.
    ///
    /// Only these are transcoded: running a charset decoder over a PNG would
    /// destroy it, so the transport leaves every other media type as bytes.
    #[must_use]
    pub fn is_textual(&self) -> bool {
        if self.type_name == "text" {
            return true;
        }
        self.type_name == "application"
            && matches!(
                self.subtype.as_str(),
                "xhtml+xml" | "xml" | "json" | "javascript" | "ecmascript"
            )
    }
}

impl fmt::Display for MediaType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.charset {
            Some(charset) => write!(
                formatter,
                "{}/{}; charset={charset}",
                self.type_name, self.subtype
            ),
            None => write!(formatter, "{}/{}", self.type_name, self.subtype),
        }
    }
}
