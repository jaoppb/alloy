//! Serialising an [`HttpRequest`] onto the wire and reading a response head
//! back off it.
//!
//! Byte-oriented and allocation-frugal, and written under the full workspace
//! lint set: no `&line[a..b]`, no raw indexing, no `as` cast, no unchecked
//! arithmetic, no `unwrap`. Every line is read through a ceiling, so a peer
//! that never sends `\n` produces [`NetworkError::LimitExceeded`] instead of
//! growing a buffer forever.
//!
//! Pure `BufRead` in, values out — no socket type appears here, which is why
//! the module is not gated behind `real-transport` and why
//! `MockTransport::from_wire` and the hostile-fixture tests can drive it
//! directly.

use std::io::{BufRead, Read};

use crate::domain::defect::{MalformedPart, WireLimit};
use crate::domain::error::NetworkError;
use crate::domain::header_map::{HeaderMap, HeaderName, HeaderValue};
use crate::domain::phase::ProtocolPhase;
use crate::domain::request::HttpRequest;
use crate::domain::status::StatusCode;
use crate::infrastructure::deadline::Deadline;
use crate::infrastructure::limits::{ByteCap, WireLimits};

/// The `User-Agent` this engine sends when the caller sets none.
pub const DEFAULT_USER_AGENT: &str = "Alloy/0.1 (+https://github.com/jaoppb/alloy)";
/// The `Accept-Encoding` this engine sends — exactly the codings
/// [`inflate`](crate::infrastructure::inflate) implements.
pub const DEFAULT_ACCEPT_ENCODING: &str = "gzip, deflate";
/// The `Accept` a page fetch sends.
pub const DEFAULT_ACCEPT: &str = "text/html,application/xhtml+xml,*/*;q=0.8";

/// Serialise a request as HTTP/1.1 bytes.
///
/// `Host` and `Content-Length` are always computed from the request itself: a
/// caller cannot set them to something that contradicts the URL or the body,
/// because that contradiction is exactly what request smuggling is made of.
#[must_use]
pub fn serialize_request(request: &HttpRequest) -> Vec<u8> {
    let fields = request_fields(request);
    let mut wire = Vec::new();
    write_request_line(&mut wire, request);
    for (name, value) in fields.iter() {
        write_field(&mut wire, name, value);
    }
    wire.extend_from_slice(b"\r\n");
    wire.extend_from_slice(request.body().as_bytes());
    wire
}

fn request_fields(request: &HttpRequest) -> HeaderMap {
    let mut fields = default_fields();
    for (name, value) in request.headers().iter() {
        fields.set(name.clone(), value.clone());
    }
    install_computed_fields(&mut fields, request);
    fields
}

fn default_fields() -> HeaderMap {
    let mut fields = HeaderMap::new();
    set_text(&mut fields, HeaderName::user_agent(), DEFAULT_USER_AGENT);
    set_text(&mut fields, HeaderName::accept(), DEFAULT_ACCEPT);
    set_text(
        &mut fields,
        HeaderName::accept_encoding(),
        DEFAULT_ACCEPT_ENCODING,
    );
    set_text(&mut fields, HeaderName::connection(), "keep-alive");
    fields
}

fn install_computed_fields(fields: &mut HeaderMap, request: &HttpRequest) {
    let url = request.url();
    let host = url.authority().to_header_text(url.scheme());
    set_text(fields, HeaderName::host(), &host);
    if request.body().is_empty() {
        fields.remove(&HeaderName::content_length());
        return;
    }
    let length = request.body().len().to_string();
    set_text(fields, HeaderName::content_length(), &length);
}

/// The value is text this crate wrote, so it carries no CR, LF or NUL and
/// [`HeaderValue::from_text`] cannot fail. Falling back to an empty value keeps
/// that fact from needing an `unwrap`.
fn set_text(fields: &mut HeaderMap, name: HeaderName, value: &str) {
    let field_value = HeaderValue::from_text(value).unwrap_or_default();
    fields.set(name, field_value);
}

fn write_request_line(wire: &mut Vec<u8>, request: &HttpRequest) {
    wire.extend_from_slice(request.method().as_str().as_bytes());
    wire.extend_from_slice(b" ");
    wire.extend_from_slice(request.url().target().to_text().as_bytes());
    wire.extend_from_slice(b" HTTP/1.1\r\n");
}

fn write_field(wire: &mut Vec<u8>, name: &HeaderName, value: &HeaderValue) {
    wire.extend_from_slice(name.as_str().as_bytes());
    wire.extend_from_slice(b": ");
    wire.extend_from_slice(value.as_bytes());
    wire.extend_from_slice(b"\r\n");
}

/// Read a status line and the field section that follows it.
///
/// A `1xx` interim response is consumed and the next status line read, as
/// RFC 9110 §15.2 requires — except `101`, which changes the protocol and is
/// therefore refused rather than skipped.
///
/// # Errors
///
/// [`NetworkError::Malformed`] for a head this parser cannot accept,
/// [`NetworkError::LimitExceeded`] for a line or a field count past its
/// ceiling, [`NetworkError::Timeout`] when the header budget runs out.
pub fn read_head(
    reader: &mut dyn BufRead,
    limits: WireLimits,
    deadline: &Deadline,
) -> Result<(StatusCode, HeaderMap), NetworkError> {
    loop {
        let status = read_status_line(reader, limits, deadline)?;
        let fields = read_field_section(reader, limits, deadline)?;
        if !status.is_informational() {
            return Ok((status, fields));
        }
        if status.code() == 101 {
            return Err(NetworkError::malformed(
                ProtocolPhase::Header,
                MalformedPart::UnsupportedTransferEncoding,
            ));
        }
    }
}

fn read_status_line(
    reader: &mut dyn BufRead,
    limits: WireLimits,
    deadline: &Deadline,
) -> Result<StatusCode, NetworkError> {
    let line = read_line(
        reader,
        limits.status_line(),
        WireLimit::StatusLineLength,
        deadline,
    )?
    .ok_or_else(|| NetworkError::malformed(ProtocolPhase::Header, MalformedPart::TruncatedHead))?;
    parse_status_line(&line)
}

fn read_field_section(
    reader: &mut dyn BufRead,
    limits: WireLimits,
    deadline: &Deadline,
) -> Result<HeaderMap, NetworkError> {
    let mut fields = HeaderMap::new();
    let mut seen = 0_usize;
    loop {
        let line = read_line(
            reader,
            limits.header_line(),
            WireLimit::HeaderLineLength,
            deadline,
        )?
        .ok_or_else(|| {
            NetworkError::malformed(ProtocolPhase::Header, MalformedPart::TruncatedHead)
        })?;
        if line.is_empty() {
            return Ok(fields);
        }
        seen = seen.saturating_add(1);
        if limits.header_count().is_exceeded_by(seen) {
            return Err(NetworkError::limit_exceeded(
                ProtocolPhase::Header,
                WireLimit::HeaderCount,
                seen,
            ));
        }
        let (name, value) = parse_field_line(&line)?;
        fields.append(name, value);
    }
}

/// Parse `HTTP/1.x <code> <reason>`.
///
/// # Errors
///
/// [`NetworkError::Malformed`] when the version or the code is not one this
/// parser accepts.
pub fn parse_status_line(line: &[u8]) -> Result<StatusCode, NetworkError> {
    let text = core::str::from_utf8(line).map_err(|_| {
        NetworkError::malformed(ProtocolPhase::Header, MalformedPart::StatusLineVersion)
    })?;
    let mut parts = text.split(' ');
    let version = parts.next().unwrap_or_default();
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(NetworkError::malformed(
            ProtocolPhase::Header,
            MalformedPart::StatusLineVersion,
        ));
    }
    let digits = parts.next().unwrap_or_default();
    let code = digits.parse::<u16>().map_err(|_| {
        NetworkError::malformed(ProtocolPhase::Header, MalformedPart::StatusLineCode)
    })?;
    StatusCode::new(code)
}

/// Parse one `name: value` field line.
///
/// Obsolete line folding (a continuation line starting with whitespace) is
/// refused, as RFC 9112 §5.2 instructs: accepting it is a request-smuggling
/// differential waiting to happen.
///
/// # Errors
///
/// [`NetworkError::Malformed`] for a missing separator, an illegal name or an
/// illegal value.
pub fn parse_field_line(line: &[u8]) -> Result<(HeaderName, HeaderValue), NetworkError> {
    if line.first().is_some_and(u8::is_ascii_whitespace) {
        return Err(NetworkError::malformed(
            ProtocolPhase::Header,
            MalformedPart::HeaderSeparator,
        ));
    }
    let separator = line.iter().position(|byte| *byte == b':').ok_or_else(|| {
        NetworkError::malformed(ProtocolPhase::Header, MalformedPart::HeaderSeparator)
    })?;
    let name_bytes = line.get(..separator).unwrap_or_default();
    let value_bytes = line.get(separator.saturating_add(1)..).unwrap_or_default();
    let name_text = core::str::from_utf8(name_bytes)
        .map_err(|_| NetworkError::malformed(ProtocolPhase::Header, MalformedPart::HeaderName))?;
    let name = HeaderName::new(name_text)?;
    let value = HeaderValue::parse(value_bytes)?;
    Ok((name, value))
}

/// Read one CRLF-terminated line, refusing to grow past `cap`.
///
/// `Ok(None)` means a clean end of stream before any byte of a line.
///
/// # Errors
///
/// [`NetworkError::LimitExceeded`] when no `\n` arrived within `cap`,
/// [`NetworkError::Transport`] for an I/O failure,
/// [`NetworkError::Timeout`] when the header budget is spent.
pub fn read_line(
    reader: &mut dyn BufRead,
    cap: ByteCap,
    limit: WireLimit,
    deadline: &Deadline,
) -> Result<Option<Vec<u8>>, NetworkError> {
    deadline.check(ProtocolPhase::Header)?;
    let ceiling = u64::try_from(cap.bytes().saturating_add(2)).unwrap_or(u64::MAX);
    let mut buffer = Vec::new();
    let mut limited = Read::take(&mut *reader, ceiling);
    let read = limited
        .read_until(b'\n', &mut buffer)
        .map_err(|error| NetworkError::transport(ProtocolPhase::Header, error.to_string()))?;
    if read == 0 {
        return Ok(None);
    }
    if !buffer.ends_with(b"\n") {
        return Err(NetworkError::limit_exceeded(
            ProtocolPhase::Header,
            limit,
            buffer.len(),
        ));
    }
    Ok(Some(trim_line_ending(&buffer)))
}

fn trim_line_ending(line: &[u8]) -> Vec<u8> {
    let without_newline = strip_last(line, b'\n');
    strip_last(without_newline, b'\r').to_vec()
}

fn strip_last(line: &[u8], byte: u8) -> &[u8] {
    if line.last() != Some(&byte) {
        return line;
    }
    line.get(..line.len().saturating_sub(1)).unwrap_or_default()
}
