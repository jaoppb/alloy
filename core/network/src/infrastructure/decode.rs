//! Turning the bytes that arrived into the [`Body`] a consumer gets.
//!
//! Two undoings, in this order and once only, inside the transport:
//!
//! 1. **`Content-Encoding`** — `gzip` and `deflate`, through
//!    [`inflate`](crate::infrastructure::inflate), under the decoded-body
//!    ceiling so a decompression bomb stops.
//! 2. **charset**, but *only for a textual media type*
//!    ([`MediaType::is_textual`]). Running a charset decoder over a PNG would
//!    destroy it, so every other media type stays bytes — which is what Phase
//!    X's `<img>` path needs.
//!
//! `core/html` therefore always receives UTF-8 and never has to guess.

use crate::domain::body::Body;
use crate::domain::defect::DecodeDefect;
use crate::domain::error::NetworkError;
use crate::domain::header_map::{HeaderMap, HeaderName};
use crate::domain::media_type::MediaType;
use crate::infrastructure::charset;
use crate::infrastructure::inflate::{self, OutputLimit};
use crate::infrastructure::limits::WireLimits;

/// Undo the content coding and, for text, the charset.
///
/// # Errors
///
/// [`NetworkError::Decode`] for an unimplemented coding or charset, a
/// malformed compressed stream, a failed container checksum, or a stream that
/// expands past the ceiling.
pub fn decode_payload(
    fields: &HeaderMap,
    raw: Vec<u8>,
    limits: WireLimits,
) -> Result<Body, NetworkError> {
    let decompressed = decompress(fields, raw, limits)?;
    let media_type = fields
        .text(&HeaderName::content_type())
        .and_then(|value| MediaType::parse(value).ok());
    transcode(&decompressed, media_type.as_ref())
}

fn decompress(
    fields: &HeaderMap,
    raw: Vec<u8>,
    limits: WireLimits,
) -> Result<Vec<u8>, NetworkError> {
    let Some(coding) = fields.text(&HeaderName::content_encoding()) else {
        return Ok(raw);
    };
    let last = coding.rsplit(',').next().unwrap_or_default().trim();
    let ceiling = OutputLimit::of_bytes(limits.decoded_body().bytes());
    if last.is_empty() || last.eq_ignore_ascii_case("identity") {
        return Ok(raw);
    }
    if last.eq_ignore_ascii_case("gzip") || last.eq_ignore_ascii_case("x-gzip") {
        return inflate::gzip_decompress_within(&raw, ceiling).map_err(NetworkError::from);
    }
    if last.eq_ignore_ascii_case("deflate") {
        return inflate_deflate_coding(&raw, ceiling);
    }
    Err(NetworkError::decode(
        DecodeDefect::UnsupportedContentEncoding,
    ))
}

/// RFC 9110 names `deflate` for the zlib framing of RFC 1950, but a long tail
/// of servers sends a bare RFC 1951 stream. Try the spec first, then the
/// common bug — the second attempt costs nothing on the success path.
fn inflate_deflate_coding(raw: &[u8], ceiling: OutputLimit) -> Result<Vec<u8>, NetworkError> {
    if let Ok(decoded) = inflate::zlib_decompress_within(raw, ceiling) {
        return Ok(decoded);
    }
    inflate::inflate_within(raw, ceiling).map_err(NetworkError::from)
}

fn transcode(bytes: &[u8], media_type: Option<&MediaType>) -> Result<Body, NetworkError> {
    let Some(media_type) = media_type else {
        return Ok(Body::from_slice(bytes));
    };
    if !media_type.is_textual() {
        return Ok(Body::from_slice(bytes));
    }
    let text = charset::decode_document(bytes, media_type.charset())?;
    Ok(Body::from_text(&text))
}
