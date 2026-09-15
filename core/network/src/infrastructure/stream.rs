//! [`NetworkStream`] — a cleartext or TLS connection, behind one `Read + Write`
//! face.
//!
//! The HTTP/1.1 layer above never branches on which it has: that is the whole
//! reason [`tls`](crate::infrastructure::tls) can be the only module in the
//! workspace that names `rustls`.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::domain::error::NetworkError;
use crate::domain::phase::ProtocolPhase;
use crate::infrastructure::tls::TlsStream;

/// A connection to a peer.
#[derive(Debug)]
pub enum NetworkStream {
    /// Cleartext over TCP.
    Plain(TcpStream),
    /// TLS over TCP. Boxed because the TLS session carries record buffers and
    /// would otherwise make every `NetworkStream` that size.
    Secure(Box<TlsStream>),
}

impl NetworkStream {
    /// The socket underneath, whichever variant this is.
    #[must_use]
    pub const fn socket(&self) -> &TcpStream {
        match self {
            Self::Plain(socket) => socket,
            Self::Secure(session) => session.socket(),
        }
    }

    /// Bound how long a single read may block.
    ///
    /// # Errors
    ///
    /// [`NetworkError::Transport`] when the operating system refuses the
    /// option — a connection with no bound is one that can hang, so this is
    /// a failure, not a warning.
    pub fn set_read_timeout(
        &self,
        budget: Duration,
        phase: ProtocolPhase,
    ) -> Result<(), NetworkError> {
        self.socket()
            .set_read_timeout(Some(budget))
            .map_err(|error| NetworkError::transport(phase, error.to_string()))
    }

    /// Bound how long a single write may block.
    ///
    /// # Errors
    ///
    /// As [`NetworkStream::set_read_timeout`].
    pub fn set_write_timeout(
        &self,
        budget: Duration,
        phase: ProtocolPhase,
    ) -> Result<(), NetworkError> {
        self.socket()
            .set_write_timeout(Some(budget))
            .map_err(|error| NetworkError::transport(phase, error.to_string()))
    }
}

impl Read for NetworkStream {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(socket) => socket.read(buffer),
            Self::Secure(session) => session.read(buffer),
        }
    }
}

impl Write for NetworkStream {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(socket) => socket.write(buffer),
            Self::Secure(session) => session.write(buffer),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(socket) => socket.flush(),
            Self::Secure(session) => session.flush(),
        }
    }
}
