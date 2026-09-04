//! The hand-written HTTP/1.1 implementation (RFC 9110 / RFC 9112).
//!
//! Split by concern rather than by message: [`message`] serialises a request
//! and parses a head, [`framing`] decides how the body is delimited and reads
//! it, [`chunked`] undoes `Transfer-Encoding: chunked`, [`exchange`] composes
//! the three into one response, and [`pool`] keeps connections alive between
//! exchanges.
//!
//! Only [`pool`] touches a socket, so it is the only child gated behind
//! `real-transport`. Everything else is `BufRead` in, values out — which is
//! what lets the hostile-fixture tests exercise the parser directly, in both
//! feature configurations.

pub mod chunked;
pub mod exchange;
pub mod framing;
pub mod message;
#[cfg(feature = "real-transport")]
pub mod pool;
