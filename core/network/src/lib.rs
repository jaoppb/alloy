//! # `network` — the HTTP transport and request-policy ports
//!
//! The network stage of the render pipeline: an [`HttpRequest`] goes in, a
//! [`RequestPolicy`] judges it, an [`HttpTransport`] performs it, and an
//! [`HttpResponse`] with a **fully decoded** body comes out — or a typed
//! [`NetworkError`] naming the [`ProtocolPhase`] it failed in. Never a hang,
//! never a panic, however hostile the peer (`PRD-009`).
//!
//! Mechanism and policy are two ports on purpose (`ADR-0011`): swapping *how*
//! bytes are fetched and deciding *whether* they may be must not be the same
//! change. Phase M installs a `.rhai` [`RequestPolicy`] on the second seam
//! without touching the first.
//!
//! This crate names **no** engine type, no `rhai`, no `dom`, no `css`, no
//! `graphics`. It depends on `thiserror` and — only in the `real-transport`
//! build — on `rustls`, `webpki-roots` and `ring`. The `layering` CI job holds
//! that line.
//!
//! ## Layout (`ADR-0010` §1)
//!
//! - [`domain`] — the two message aggregates ([`HttpRequest`],
//!   [`HttpResponse`]), the value objects ([`Url`], [`HeaderMap`],
//!   [`StatusCode`], [`Method`], [`MediaType`], [`Body`]), the typed
//!   [`NetworkError`] with its [`ProtocolPhase`] location and its defect
//!   enums. Zero I/O.
//! - [`application`] — the two ports ([`HttpTransport`], [`RequestPolicy`],
//!   [`PolicyVerdict`]) and the [`conformance`] suite.
//! - [`infrastructure`] — the hand-written HTTP/1.1 client, the `rustls`/`ring`
//!   TLS layer, the RFC 1951 [`inflate`](infrastructure::inflate) codec, the
//!   charset decoder, the redirect rules, and the in-repo
//!   [`MockTransport`] / [`AllowAllPolicy`].
//!
//! ## Threading (`ADR-0019`)
//!
//! [`HttpTransport`] is **synchronous** and blocking. The window event loop
//! owns the main thread; a consumer runs `execute` on a `std::thread` pool
//! worker and takes the [`HttpResponse`] back over `std::sync::mpsc` as one
//! more loop event. There is no async runtime in this workspace.
//!
//! ## `unsafe` (`ADR-0018`)
//!
//! This crate is `#![forbid(unsafe_code)]`, the HTTP/1.1 parser and the
//! `inflate` codec included — they read attacker-controlled bytes, which is
//! `ADR-0018` row 1. The single third-party exception is `ring`, the `rustls`
//! crypto provider chosen by the Phase C0 spike
//! (`docs/reports/SPIKE-C0-TLS-PROVIDER.md` §6) under the pre-authorised
//! carve-out, recorded as the only row-1 entry in `unsafe-allowlist.toml`.
//!
//! ## Contract record
//!
//! This crate is the `HttpTransport` / `RequestPolicy` port under the
//! `ADR-0011` Replaceable Port Contract. The message aggregates and
//! [`PORT_SCHEMA_VERSION`] **freeze at integration point I4**;
//! `docs/architecture/http-transport-port-contract.md` records the state of all
//! seven items from that point on. A change after the freeze also needs a
//! migration note in `PRD-009`.

#![forbid(unsafe_code)]
// Every fallible function documents its failures through the typed
// `NetworkError` variant it returns; a prose `# Errors` section on each would
// restate the enum. Same call, same reason, as `core/dom/src/lib.rs:24` and
// `core/css/src/lib.rs:44`.
#![allow(clippy::missing_errors_doc)]

pub mod application;
pub mod domain;
pub mod infrastructure;

/// The observable version of this port's message aggregates.
///
/// `ADR-0011` item 3. Bumped on any change a transport, a policy or a consumer
/// could notice; **frozen at I4**, after which a change also needs a migration
/// note in `PRD-009`.
pub const PORT_SCHEMA_VERSION: u32 = 1;

/// The content codings [`decode_payload`](infrastructure::decode::decode_payload)
/// can undo — exactly what the request's `Accept-Encoding` advertises.
///
/// A response coded in anything else is a typed
/// [`DecodeDefect::UnsupportedContentEncoding`], never a body handed on as
/// compressed noise.
pub const SUPPORTED_CONTENT_ENCODINGS: [&str; 3] = ["identity", "gzip", "deflate"];

/// The charsets [`charset`](infrastructure::charset) can decode. Everything
/// else is a typed [`DecodeDefect::UnsupportedCharset`].
pub const SUPPORTED_CHARSETS: [&str; 2] = ["utf-8", "windows-1252"];

pub use application::conformance;
pub use application::ports::{HttpTransport, PolicyVerdict, RequestPolicy};
pub use domain::authority::{Authority, Host, Port};
pub use domain::body::Body;
pub use domain::defect::{
    DecodeDefect, FramingDefect, MalformedPart, RedirectDefect, UrlDefect, WireLimit,
};
pub use domain::error::NetworkError;
pub use domain::header_map::{HeaderMap, HeaderName, HeaderValue};
pub use domain::media_type::{Charset, MediaType};
pub use domain::method::Method;
pub use domain::phase::ProtocolPhase;
pub use domain::request::HttpRequest;
pub use domain::response::HttpResponse;
pub use domain::scheme::Scheme;
pub use domain::status::StatusCode;
pub use domain::target::{Path, Query, RequestTarget};
pub use domain::url::Url;
pub use infrastructure::deadline::{Deadline, PhaseTimeouts};
pub use infrastructure::inflate;
pub use infrastructure::limits::{ByteCap, FieldCap, WireLimits};
pub use infrastructure::mock::{AllowAllPolicy, MockTransport};
pub use infrastructure::redirect::{RedirectLimit, RedirectTrail};

#[cfg(feature = "real-transport")]
pub use infrastructure::real_transport::RealHttpTransport;
