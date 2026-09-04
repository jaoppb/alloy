//! The adapters behind the two ports (`ADR-0010` §1).
//!
//! ## What `real-transport` gates, and why
//!
//! The rule is precise: **anything that opens a socket or links crypto is
//! gated; a pure byte transform is not.**
//!
//! - Gated ([`dns`], [`tls`], [`stream`], [`http1::pool`],
//!   [`real_transport`]): `std::net`, `rustls`, `webpki-roots`, `ring`.
//! - Ungated ([`inflate`], [`charset`], [`decode`], [`redirect`],
//!   [`limits`], [`deadline`], the rest of [`http1`], [`mock`]): pure
//!   `&[u8] -> Result<_, NetworkError>` logic that links nothing.
//!
//! So `--no-default-features` — the `no-transport` build of `ADR-0011` item 6 —
//! links no socket and no crypto code, exactly as promised, while still
//! carrying the parsers the hostile-fixture tests must exercise and the
//! [`inflate`] Phase X re-exports for its PNG decoder.

pub mod charset;
pub mod deadline;
pub mod decode;
pub mod http1;
pub mod inflate;
pub mod limits;
pub mod mock;
pub mod redirect;

#[cfg(feature = "real-transport")]
pub mod dns;
#[cfg(feature = "real-transport")]
pub mod real_transport;
#[cfg(feature = "real-transport")]
pub mod stream;
#[cfg(feature = "real-transport")]
pub mod tls;

pub use mock::{AllowAllPolicy, MockTransport};

#[cfg(feature = "real-transport")]
pub use real_transport::RealHttpTransport;
