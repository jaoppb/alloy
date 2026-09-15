//! The **only** file in this workspace that names `rustls`, `ring` or
//! `webpki_roots`.
//!
//! `ADR-0011` item 2 / `PRD-009` item 2: no TLS type may appear in a port
//! signature. [`TlsConnector::connect`] therefore hands back a [`TlsStream`]
//! that is nothing but `Read + Write` to everything above it — the HTTP/1.1
//! layer never learns whether it is talking through TLS.
//!
//! ## The `ring` carve-out (`ADR-0018` row 1)
//!
//! The crypto provider is `ring`, chosen by the Phase C0 spike
//! (`docs/reports/SPIKE-C0-TLS-PROVIDER.md` §6) under the pre-authorised
//! carve-out in `ADR-0018` §"The `RustCrypto` carve-out". The pure-Rust
//! alternative was NO-GO: `rustls-rustcrypto` is `0.0.2-alpha` with no stable
//! release, forces a `deny.toml`-banned duplicate `rustls-webpki`, and is not
//! itself `unsafe`-free. The single allowlist entry is recorded in
//! `unsafe-allowlist.toml`.
//!
//! The provider is passed **explicitly** through
//! [`ClientConfig::builder_with_provider`]. The no-argument
//! `ClientConfig::builder()` is never used: it picks a provider from a
//! compile-time default and hides the choice, which is the one thing this
//! carve-out must not do.
//!
//! ## Trust roots
//!
//! Embedded `webpki-roots` — the Mozilla CA set compiled in — **never** the
//! operating system trust store. That keeps certificate validation identical on
//! all three OSes and makes the trust set a reviewable dependency bump rather
//! than a property of whatever machine the browser happens to run on.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

use crate::domain::authority::Host;
use crate::domain::error::NetworkError;

// The ADR-0018 row-1 EXCEPTION, named here so `cargo tree -p network` and this
// file agree about where the assembly `unsafe` enters the build. `ring` is
// reached through `rustls::crypto::ring`, so this import exists to make the
// dependency honest rather than to call it.
use ring as _;

/// Builds TLS client connections from one shared, validated configuration.
#[derive(Clone, Debug)]
pub struct TlsConnector {
    config: Arc<ClientConfig>,
}

impl TlsConnector {
    /// Build the client configuration: the `ring` provider, the embedded
    /// Mozilla roots, safe default protocol versions, no client certificate.
    ///
    /// # Errors
    ///
    /// [`NetworkError::HandshakeRejected`] when `rustls` refuses the provider
    /// and version combination — a build-configuration fault, surfaced as a
    /// typed error rather than a panic.
    pub fn new() -> Result<Self, NetworkError> {
        let provider = rustls::crypto::ring::default_provider();
        let roots = RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };
        let config = ClientConfig::builder_with_provider(Arc::new(provider))
            .with_safe_default_protocol_versions()
            .map_err(|error| {
                NetworkError::handshake_rejected("<client configuration>", error.to_string())
            })?
            .with_root_certificates(roots)
            .with_no_client_auth();
        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// Complete a TLS handshake over an already-connected socket.
    ///
    /// The handshake is driven to completion **here** rather than lazily on the
    /// first read, so a rejected certificate is a
    /// [`NetworkError::HandshakeRejected`] in the handshake phase instead of a
    /// confusing header-phase failure.
    ///
    /// # Errors
    ///
    /// [`NetworkError::HandshakeRejected`] when the name is not a valid server
    /// name, the session cannot be created, or the peer's chain is refused.
    pub fn connect(&self, host: &Host, mut socket: TcpStream) -> Result<TlsStream, NetworkError> {
        let name = ServerName::try_from(host.as_str().to_owned())
            .map_err(|error| NetworkError::handshake_rejected(host.as_str(), error.to_string()))?;
        let mut connection = ClientConnection::new(Arc::clone(&self.config), name)
            .map_err(|error| NetworkError::handshake_rejected(host.as_str(), error.to_string()))?;
        while connection.is_handshaking() {
            connection.complete_io(&mut socket).map_err(|error| {
                NetworkError::handshake_rejected(host.as_str(), error.to_string())
            })?;
        }
        Ok(TlsStream(StreamOwned::new(connection, socket)))
    }
}

/// An established TLS session over a TCP socket.
///
/// Opaque on purpose: outside this module it is only `Read + Write` plus a
/// borrow of the socket for timeout control.
#[derive(Debug)]
pub struct TlsStream(StreamOwned<ClientConnection, TcpStream>);

impl TlsStream {
    /// The socket underneath, for read and write timeouts.
    #[must_use]
    pub const fn socket(&self) -> &TcpStream {
        &self.0.sock
    }
}

impl Read for TlsStream {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buffer)
    }
}

impl Write for TlsStream {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.0.write(buffer)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}
