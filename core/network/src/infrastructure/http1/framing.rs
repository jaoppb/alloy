//! Deciding how a response body is delimited, and reading it under a ceiling
//! and a deadline.
//!
//! RFC 9112 §6: the framing is chosen by the status, the method, and then
//! `Transfer-Encoding` **before** `Content-Length` — never both, because
//! honouring both is precisely the request-smuggling differential.

use std::io::BufRead;

use crate::domain::defect::{FramingDefect, MalformedPart, WireLimit};
use crate::domain::error::NetworkError;
use crate::domain::header_map::{HeaderMap, HeaderName};
use crate::domain::method::Method;
use crate::domain::phase::ProtocolPhase;
use crate::domain::status::StatusCode;
use crate::infrastructure::deadline::Deadline;
use crate::infrastructure::http1::chunked;
use crate::infrastructure::limits::WireLimits;

/// How many bytes one read call asks for. Small enough that the deadline is
/// checked often on a dribbling connection, large enough not to syscall per
/// byte.
const READ_CHUNK: usize = 16 * 1024;

/// How a response body is delimited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BodyFraming {
    /// The status or the method forbids a body outright (RFC 9112 §6.3).
    Empty,
    /// `Content-Length` bytes follow.
    Length(usize),
    /// `Transfer-Encoding: chunked`.
    Chunked,
    /// Neither: the body runs to the end of the connection.
    UntilClose,
}

/// Decide the framing of the body that follows a head.
///
/// # Errors
///
/// [`NetworkError::Malformed`] for a non-numeric or self-contradictory
/// `Content-Length`, or a `Transfer-Encoding` this engine does not implement;
/// [`NetworkError::LimitExceeded`] when the declared length is past the body
/// ceiling.
pub fn framing_of(
    status: StatusCode,
    method: Method,
    fields: &HeaderMap,
    limits: WireLimits,
) -> Result<BodyFraming, NetworkError> {
    if status.forbids_body() || !method.allows_response_body() {
        return Ok(BodyFraming::Empty);
    }
    if let Some(coding) = fields.text(&HeaderName::transfer_encoding()) {
        return transfer_encoded_framing(coding);
    }
    let Some(declared) = fields.text(&HeaderName::content_length()) else {
        return Ok(BodyFraming::UntilClose);
    };
    declared_length_framing(declared, limits)
}

fn transfer_encoded_framing(coding: &str) -> Result<BodyFraming, NetworkError> {
    let last = coding.rsplit(',').next().unwrap_or_default().trim();
    if last.eq_ignore_ascii_case("chunked") {
        return Ok(BodyFraming::Chunked);
    }
    Err(NetworkError::malformed(
        ProtocolPhase::Header,
        MalformedPart::UnsupportedTransferEncoding,
    ))
}

/// A repeated `Content-Length` arrives here combined as `"5, 5"` by
/// [`HeaderMap::append`]. Identical values are the same length said twice;
/// differing ones are a smuggling attempt.
fn declared_length_framing(
    declared: &str,
    limits: WireLimits,
) -> Result<BodyFraming, NetworkError> {
    let mut lengths = declared
        .split(',')
        .map(|value| value.trim().parse::<usize>());
    let first = lengths
        .next()
        .unwrap_or(Ok(0))
        .map_err(|_| NetworkError::malformed(ProtocolPhase::Header, MalformedPart::HeaderNumber))?;
    for further in lengths {
        let value = further.map_err(|_| {
            NetworkError::malformed(ProtocolPhase::Header, MalformedPart::HeaderNumber)
        })?;
        if value != first {
            return Err(NetworkError::malformed(
                ProtocolPhase::Header,
                MalformedPart::ContradictoryContentLength,
            ));
        }
    }
    if limits.body().is_exceeded_by(first) {
        return Err(NetworkError::limit_exceeded(
            ProtocolPhase::Body,
            WireLimit::BodyLength,
            first,
        ));
    }
    Ok(BodyFraming::Length(first))
}

/// Read the body a head announced.
///
/// # Errors
///
/// [`NetworkError::Framing`] when the peer stops short of what it declared,
/// [`NetworkError::LimitExceeded`] at the ceiling, [`NetworkError::Timeout`]
/// when the body budget is spent, [`NetworkError::Transport`] for an I/O
/// failure.
pub fn read_body(
    reader: &mut dyn BufRead,
    framing: BodyFraming,
    limits: WireLimits,
    deadline: &Deadline,
) -> Result<Vec<u8>, NetworkError> {
    match framing {
        BodyFraming::Empty => Ok(Vec::new()),
        BodyFraming::Length(length) => read_exactly(reader, length, deadline),
        BodyFraming::Chunked => chunked::decode(reader, limits, deadline),
        BodyFraming::UntilClose => read_until_close(reader, limits, deadline),
    }
}

/// Read exactly `count` bytes, or fail because the peer lied about how many
/// there were.
///
/// # Errors
///
/// [`NetworkError::Framing`] with [`FramingDefect::BodyShorterThanDeclared`]
/// on an early end of stream.
pub fn read_exactly(
    reader: &mut dyn BufRead,
    count: usize,
    deadline: &Deadline,
) -> Result<Vec<u8>, NetworkError> {
    let mut collected: Vec<u8> = Vec::new();
    let mut buffer = vec![0_u8; READ_CHUNK];
    while collected.len() < count {
        deadline.check(ProtocolPhase::Body)?;
        let wanted = count.saturating_sub(collected.len()).min(READ_CHUNK);
        let window = buffer
            .get_mut(..wanted)
            .ok_or_else(|| NetworkError::framing(FramingDefect::BodyShorterThanDeclared))?;
        let read = reader
            .read(window)
            .map_err(|error| NetworkError::transport(ProtocolPhase::Body, error.to_string()))?;
        if read == 0 {
            return Err(NetworkError::framing(
                FramingDefect::BodyShorterThanDeclared,
            ));
        }
        let filled = window
            .get(..read)
            .ok_or_else(|| NetworkError::framing(FramingDefect::BodyShorterThanDeclared))?;
        collected.extend_from_slice(filled);
    }
    Ok(collected)
}

fn read_until_close(
    reader: &mut dyn BufRead,
    limits: WireLimits,
    deadline: &Deadline,
) -> Result<Vec<u8>, NetworkError> {
    let mut collected: Vec<u8> = Vec::new();
    let mut buffer = vec![0_u8; READ_CHUNK];
    loop {
        deadline.check(ProtocolPhase::Body)?;
        let read = reader
            .read(&mut buffer)
            .map_err(|error| NetworkError::transport(ProtocolPhase::Body, error.to_string()))?;
        if read == 0 {
            return Ok(collected);
        }
        let filled = buffer.get(..read).unwrap_or_default();
        collected.extend_from_slice(filled);
        if limits.body().is_exceeded_by(collected.len()) {
            return Err(NetworkError::limit_exceeded(
                ProtocolPhase::Body,
                WireLimit::BodyLength,
                collected.len(),
            ));
        }
    }
}
