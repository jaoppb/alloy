//! The typed sub-reasons a [`NetworkError`] carries.
//!
//! A hostile server is described by an enum, not by a sentence: a test asserts
//! `FramingDefect::ChunkSizeNotHexadecimal`, never `reason.contains("chunk")`.
//! Prose belongs in `Display`; the discriminant is the contract
//! (`ADR-0011` item 4).
//!
//! [`NetworkError`]: crate::domain::error::NetworkError

use core::fmt;

/// Why a string could not become a [`Url`](crate::domain::url::Url).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UrlDefect {
    /// No `scheme://` prefix at all.
    MissingScheme,
    /// A scheme this engine does not speak (only `http` and `https` exist here).
    UnsupportedScheme,
    /// The authority was empty — `http:///path` names no host.
    MissingHost,
    /// The host carried a character no host may carry.
    MalformedHost,
    /// The port was absent after `:`, non-numeric, zero, or above `65535`.
    MalformedPort,
    /// The path or query carried a control character or a raw space.
    MalformedPath,
    /// `user:password@host` — credentials in a URL are refused outright.
    EmbeddedCredentials,
    /// A relative reference was resolved against nothing usable.
    UnresolvableReference,
}

impl UrlDefect {
    /// A short human sentence for `Display`.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::MissingScheme => "no scheme:// prefix",
            Self::UnsupportedScheme => "scheme is neither http nor https",
            Self::MissingHost => "the authority names no host",
            Self::MalformedHost => "the host carries an illegal character",
            Self::MalformedPort => "the port is absent, non-numeric, zero or out of range",
            Self::MalformedPath => "the path or query carries a control character",
            Self::EmbeddedCredentials => "credentials embedded in the authority are refused",
            Self::UnresolvableReference => "the reference cannot be resolved against this base",
        }
    }
}

impl fmt::Display for UrlDefect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason())
    }
}

/// Which part of the head the peer got wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MalformedPart {
    /// The status line did not start with a recognised HTTP version.
    StatusLineVersion,
    /// The three status digits were absent or outside `100..=599`.
    StatusLineCode,
    /// A header line carried no `:` separator.
    HeaderSeparator,
    /// A header name was empty or carried a non-token character.
    HeaderName,
    /// A header value carried a bare CR, LF or NUL (response splitting).
    HeaderValue,
    /// A header whose value must be a number was not one.
    HeaderNumber,
    /// Two `Content-Length` headers that disagree.
    ContradictoryContentLength,
    /// A `Transfer-Encoding` this engine does not implement.
    UnsupportedTransferEncoding,
    /// The head ended before the terminating empty line.
    TruncatedHead,
}

impl MalformedPart {
    /// A short human sentence for `Display`.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::StatusLineVersion => "the status line names no HTTP/1.x version",
            Self::StatusLineCode => "the status code is missing or out of range",
            Self::HeaderSeparator => "a header line carries no ':' separator",
            Self::HeaderName => "a header name is empty or not a token",
            Self::HeaderValue => "a header value carries a bare CR, LF or NUL",
            Self::HeaderNumber => "a numeric header is not a number",
            Self::ContradictoryContentLength => "two Content-Length headers disagree",
            Self::UnsupportedTransferEncoding => "the Transfer-Encoding is not implemented",
            Self::TruncatedHead => "the head ended before its terminating empty line",
        }
    }
}

impl fmt::Display for MalformedPart {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason())
    }
}

/// How the peer framed the body wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FramingDefect {
    /// The connection ended before `Content-Length` bytes arrived — a lying
    /// `Content-Length`.
    BodyShorterThanDeclared,
    /// A chunk size line was not hexadecimal.
    ChunkSizeNotHexadecimal,
    /// A chunk size line was empty.
    ChunkSizeMissing,
    /// A chunk was not followed by its CRLF.
    ChunkTerminatorMissing,
    /// The stream ended before the terminating zero-length chunk.
    FinalChunkMissing,
    /// The connection dropped mid-body with no framing that allows it.
    ConnectionClosedEarly,
}

impl FramingDefect {
    /// A short human sentence for `Display`.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::BodyShorterThanDeclared => "the body is shorter than Content-Length declared",
            Self::ChunkSizeNotHexadecimal => "a chunk size line is not hexadecimal",
            Self::ChunkSizeMissing => "a chunk size line is empty",
            Self::ChunkTerminatorMissing => "a chunk is not followed by CRLF",
            Self::FinalChunkMissing => "the stream ended before the zero-length chunk",
            Self::ConnectionClosedEarly => "the connection closed mid-body",
        }
    }
}

impl fmt::Display for FramingDefect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason())
    }
}

/// Why a redirect chain was abandoned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RedirectDefect {
    /// The chain revisited a URL it had already been sent to.
    Cycle,
    /// The chain outlived its hop limit.
    LimitExceeded,
    /// A 3xx response carried no `Location`.
    LocationMissing,
    /// The `Location` did not resolve to a usable absolute URL.
    LocationUnresolvable,
}

impl RedirectDefect {
    /// A short human sentence for `Display`.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::Cycle => "the redirect chain revisits a URL",
            Self::LimitExceeded => "the redirect chain outlived its hop limit",
            Self::LocationMissing => "a 3xx response carries no Location",
            Self::LocationUnresolvable => "the Location does not resolve to an absolute URL",
        }
    }
}

impl fmt::Display for RedirectDefect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason())
    }
}

/// Why a body could not be turned into bytes or text the caller can use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DecodeDefect {
    /// A `Content-Encoding` this engine does not implement.
    UnsupportedContentEncoding,
    /// The compressed stream is malformed or truncated.
    MalformedCompressedStream,
    /// The compressed stream expands past the output ceiling — a zip bomb.
    CompressionRatioTooHigh,
    /// A checksum in the container (gzip CRC32, zlib Adler-32) disagreed.
    ChecksumMismatch,
    /// A charset label this engine does not implement.
    UnsupportedCharset,
    /// A byte-order mark announced an encoding this engine does not implement.
    UnsupportedByteOrderMark,
    /// A `Content-Type` header that is not a media type.
    MalformedMediaType,
}

impl DecodeDefect {
    /// A short human sentence for `Display`.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::UnsupportedContentEncoding => "the Content-Encoding is not implemented",
            Self::MalformedCompressedStream => "the compressed stream is malformed or truncated",
            Self::CompressionRatioTooHigh => "the compressed stream expands past the ceiling",
            Self::ChecksumMismatch => "a container checksum disagreed",
            Self::UnsupportedCharset => "the charset label is not implemented",
            Self::UnsupportedByteOrderMark => "the byte-order mark names an unimplemented encoding",
            Self::MalformedMediaType => "the Content-Type is not a media type",
        }
    }
}

impl fmt::Display for DecodeDefect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason())
    }
}

/// Which ceiling a hostile peer pushed past.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WireLimit {
    /// The status line grew past its byte cap.
    StatusLineLength,
    /// One header line grew past its byte cap.
    HeaderLineLength,
    /// The header block carried more lines than the cap allows.
    HeaderCount,
    /// The body grew — or declared it would grow — past its byte cap.
    BodyLength,
    /// A chunk size line grew past its byte cap.
    ChunkLineLength,
    /// A decompressed body grew past its byte cap.
    DecodedLength,
}

impl WireLimit {
    /// A short human sentence for `Display`.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::StatusLineLength => "the status line is longer than allowed",
            Self::HeaderLineLength => "a header line is longer than allowed",
            Self::HeaderCount => "the header block has more lines than allowed",
            Self::BodyLength => "the body is longer than allowed",
            Self::ChunkLineLength => "a chunk size line is longer than allowed",
            Self::DecodedLength => "the decoded body is longer than allowed",
        }
    }
}

impl fmt::Display for WireLimit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason())
    }
}
