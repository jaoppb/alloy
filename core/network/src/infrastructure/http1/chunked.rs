//! The `Transfer-Encoding: chunked` decoder (RFC 9112 §7.1).
//!
//! Every one of the four things a hostile peer can do to a chunked stream has
//! a named answer here: a size line that is not hexadecimal, a size line that
//! is empty, a chunk not followed by its CRLF, and a stream that ends before
//! the terminating zero-length chunk. Each returns a typed
//! [`FramingDefect`]; none can loop forever, because the running total is
//! checked against the body ceiling and the deadline is checked once per
//! chunk.

use std::io::BufRead;

use crate::domain::defect::{FramingDefect, WireLimit};
use crate::domain::error::NetworkError;
use crate::domain::phase::ProtocolPhase;
use crate::infrastructure::deadline::Deadline;
use crate::infrastructure::http1::framing::read_exactly;
use crate::infrastructure::http1::message::read_line;
use crate::infrastructure::limits::WireLimits;

/// Read a chunked body to its terminating zero-length chunk.
///
/// # Errors
///
/// [`NetworkError::Framing`] for every malformation above,
/// [`NetworkError::LimitExceeded`] at the body or chunk-line ceiling,
/// [`NetworkError::Timeout`] when the body budget is spent.
pub fn decode(
    reader: &mut dyn BufRead,
    limits: WireLimits,
    deadline: &Deadline,
) -> Result<Vec<u8>, NetworkError> {
    let mut collected: Vec<u8> = Vec::new();
    loop {
        deadline.check(ProtocolPhase::Body)?;
        let size = read_chunk_size(reader, limits, deadline)?;
        if size == 0 {
            read_trailer_section(reader, limits, deadline)?;
            return Ok(collected);
        }
        guard_total(collected.len(), size, limits)?;
        let chunk = read_exactly(reader, size, deadline)?;
        collected.extend_from_slice(&chunk);
        expect_chunk_terminator(reader, limits, deadline)?;
    }
}

fn read_chunk_size(
    reader: &mut dyn BufRead,
    limits: WireLimits,
    deadline: &Deadline,
) -> Result<usize, NetworkError> {
    let line = read_line(
        reader,
        limits.chunk_line(),
        WireLimit::ChunkLineLength,
        deadline,
    )?
    .ok_or_else(|| NetworkError::framing(FramingDefect::FinalChunkMissing))?;
    parse_chunk_size(&line)
}

/// `size[;extension]` in hexadecimal. Extensions are read and discarded — this
/// engine defines none, and RFC 9112 §7.1.1 says an unknown one is ignored.
fn parse_chunk_size(line: &[u8]) -> Result<usize, NetworkError> {
    let text = core::str::from_utf8(line)
        .map_err(|_| NetworkError::framing(FramingDefect::ChunkSizeNotHexadecimal))?;
    let digits = text.split(';').next().unwrap_or_default().trim();
    if digits.is_empty() {
        return Err(NetworkError::framing(FramingDefect::ChunkSizeMissing));
    }
    usize::from_str_radix(digits, 16)
        .map_err(|_| NetworkError::framing(FramingDefect::ChunkSizeNotHexadecimal))
}

fn guard_total(collected: usize, incoming: usize, limits: WireLimits) -> Result<(), NetworkError> {
    let total = collected.checked_add(incoming).ok_or_else(|| {
        NetworkError::limit_exceeded(ProtocolPhase::Body, WireLimit::BodyLength, collected)
    })?;
    if limits.body().is_exceeded_by(total) {
        return Err(NetworkError::limit_exceeded(
            ProtocolPhase::Body,
            WireLimit::BodyLength,
            total,
        ));
    }
    Ok(())
}

fn expect_chunk_terminator(
    reader: &mut dyn BufRead,
    limits: WireLimits,
    deadline: &Deadline,
) -> Result<(), NetworkError> {
    let line = read_line(
        reader,
        limits.chunk_line(),
        WireLimit::ChunkLineLength,
        deadline,
    )?
    .ok_or_else(|| NetworkError::framing(FramingDefect::ChunkTerminatorMissing))?;
    if line.is_empty() {
        return Ok(());
    }
    Err(NetworkError::framing(FramingDefect::ChunkTerminatorMissing))
}

/// The trailer section after the zero-length chunk: field lines until an empty
/// one. Its fields are discarded — nothing downstream reads them yet — but it
/// is still read under the field-count ceiling so it cannot be an unbounded
/// stream in disguise.
fn read_trailer_section(
    reader: &mut dyn BufRead,
    limits: WireLimits,
    deadline: &Deadline,
) -> Result<(), NetworkError> {
    let mut seen = 0_usize;
    loop {
        let Some(line) = read_line(
            reader,
            limits.header_line(),
            WireLimit::HeaderLineLength,
            deadline,
        )?
        else {
            return Ok(());
        };
        if line.is_empty() {
            return Ok(());
        }
        seen = seen.saturating_add(1);
        if limits.header_count().is_exceeded_by(seen) {
            return Err(NetworkError::limit_exceeded(
                ProtocolPhase::Body,
                WireLimit::HeaderCount,
                seen,
            ));
        }
    }
}
