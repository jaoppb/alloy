//! Reading one complete HTTP/1.1 response off a reader.
//!
//! The composition point: head, framing, body, content coding, charset. It
//! takes a `&mut dyn BufRead`, not a socket, so the same code path serves the
//! real transport, `MockTransport::with_wire_response`, and every hostile
//! fixture in `core/network/tests/hostile_responses.rs`.

use std::io::BufRead;

use crate::domain::error::NetworkError;
use crate::domain::header_map::{HeaderName, HeaderValue};
use crate::domain::method::Method;
use crate::domain::response::HttpResponse;
use crate::infrastructure::deadline::Deadline;
use crate::infrastructure::decode::decode_payload;
use crate::infrastructure::http1::framing::{framing_of, read_body};
use crate::infrastructure::http1::message::read_head;
use crate::infrastructure::limits::WireLimits;

/// Read a status line, a field section and a body, and hand back a decoded
/// response.
///
/// # Errors
///
/// Every failure class of this port: [`NetworkError::Malformed`],
/// [`NetworkError::Framing`], [`NetworkError::LimitExceeded`],
/// [`NetworkError::Decode`], [`NetworkError::Timeout`],
/// [`NetworkError::Transport`].
pub fn read_response(
    reader: &mut dyn BufRead,
    method: Method,
    limits: WireLimits,
    deadline: &Deadline,
) -> Result<HttpResponse, NetworkError> {
    let (status, fields) = read_head(reader, limits, deadline)?;
    let framing = framing_of(status, method, &fields, limits)?;
    let raw = read_body(reader, framing, limits, deadline)?;
    let body = decode_payload(&fields, raw, limits)?;
    Ok(HttpResponse::new(status, fields, body))
}

/// Whether the peer agreed to keep this connection open for another exchange.
///
/// HTTP/1.1 defaults to keep-alive, so the question is only whether anyone
/// said `close`.
#[must_use]
pub fn connection_is_reusable(response: &HttpResponse) -> bool {
    let Some(value) = response.headers().get(&HeaderName::connection()) else {
        return true;
    };
    !names_close(value)
}

fn names_close(value: &HeaderValue) -> bool {
    let Some(text) = value.as_str() else {
        return true;
    };
    text.split(',')
        .any(|token| token.trim().eq_ignore_ascii_case("close"))
}
