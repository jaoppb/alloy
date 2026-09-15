//! [`RealHttpTransport`] — the reference adapter of `ADR-0011` item 6: the
//! hand-written HTTP/1.1 client over `std::net`, with TLS from
//! [`tls`](crate::infrastructure::tls).
//!
//! It composes the pieces and owns nothing but the composition: DNS, connect,
//! handshake, serialise, read, decode, redirect. Every one of those steps runs
//! under a phase timeout *and* the whole exchange runs under a total deadline,
//! because a peer that dribbles bytes forever trips neither read timeout alone.
//!
//! `execute` blocks. Per `ADR-0019` a consumer runs it on a `std::thread` pool
//! worker and takes the result back over `mpsc`; there is no async runtime in
//! this workspace and this trait does not introduce one.

use std::io::{BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use crate::application::ports::HttpTransport;
use crate::domain::error::NetworkError;
use crate::domain::phase::ProtocolPhase;
use crate::domain::request::HttpRequest;
use crate::domain::response::HttpResponse;
use crate::domain::url::Url;
use crate::infrastructure::deadline::{Deadline, PhaseTimeouts};
use crate::infrastructure::dns;
use crate::infrastructure::http1::exchange::{connection_is_reusable, read_response};
use crate::infrastructure::http1::message::serialize_request;
use crate::infrastructure::http1::pool::{ConnectionPool, PoolKey};
use crate::infrastructure::limits::WireLimits;
use crate::infrastructure::redirect::{self, RedirectLimit, RedirectTrail};
use crate::infrastructure::stream::NetworkStream;
use crate::infrastructure::tls::TlsConnector;

/// The reference [`HttpTransport`]: real sockets, real TLS.
#[derive(Debug)]
pub struct RealHttpTransport {
    connector: TlsConnector,
    pool: ConnectionPool,
    timeouts: PhaseTimeouts,
    limits: WireLimits,
    redirects: RedirectLimit,
}

impl RealHttpTransport {
    /// Build a transport with the default budgets, ceilings and redirect limit.
    ///
    /// # Errors
    ///
    /// [`NetworkError::HandshakeRejected`] when the TLS client configuration
    /// cannot be built.
    pub fn new() -> Result<Self, NetworkError> {
        Ok(Self {
            connector: TlsConnector::new()?,
            pool: ConnectionPool::new(),
            timeouts: PhaseTimeouts::DEFAULT,
            limits: WireLimits::DEFAULT,
            redirects: RedirectLimit::DEFAULT,
        })
    }

    /// The same transport under different phase budgets.
    #[must_use]
    pub const fn with_timeouts(mut self, timeouts: PhaseTimeouts) -> Self {
        self.timeouts = timeouts;
        self
    }

    /// The same transport under different wire ceilings.
    #[must_use]
    pub const fn with_limits(mut self, limits: WireLimits) -> Self {
        self.limits = limits;
        self
    }

    /// The same transport under a different redirect hop limit.
    #[must_use]
    pub const fn with_redirect_limit(mut self, redirects: RedirectLimit) -> Self {
        self.redirects = redirects;
        self
    }

    /// How many idle pooled connections are held.
    #[must_use]
    pub fn idle_connections(&self) -> usize {
        self.pool.idle_count()
    }

    /// One request, one response — redirects not followed.
    ///
    /// A pooled connection is tried first; if the exchange over it fails, the
    /// socket was stale and one fresh attempt is made. A failure on the fresh
    /// attempt is the caller's answer.
    fn exchange(
        &self,
        request: &HttpRequest,
        deadline: &Deadline,
    ) -> Result<HttpResponse, NetworkError> {
        let key = PoolKey::of(request.url());
        if let Some(pooled) = self.pool.checkout(&key)
            && let Ok(response) = self.exchange_over(pooled, request, deadline, &key)
        {
            return Ok(response);
        }
        let fresh = self.connect(request.url(), deadline)?;
        self.exchange_over(fresh, request, deadline, &key)
    }

    fn exchange_over(
        &self,
        stream: NetworkStream,
        request: &HttpRequest,
        deadline: &Deadline,
        key: &PoolKey,
    ) -> Result<HttpResponse, NetworkError> {
        let mut stream = stream;
        self.arm_timeouts(&stream, deadline)?;
        write_request(&mut stream, request)?;
        let mut reader = BufReader::new(stream);
        let response = read_response(&mut reader, request.method(), self.limits, deadline)?;
        self.return_to_pool(reader, &response, key);
        Ok(response)
    }

    fn arm_timeouts(
        &self,
        stream: &NetworkStream,
        deadline: &Deadline,
    ) -> Result<(), NetworkError> {
        let header = clamp(self.timeouts.header(), deadline);
        let body = clamp(self.timeouts.body(), deadline);
        stream.set_write_timeout(header, ProtocolPhase::Header)?;
        stream.set_read_timeout(body.max(header), ProtocolPhase::Body)
    }

    /// A connection is reusable only when the peer did not say `close` **and**
    /// nothing is left buffered: leftover bytes would become the first bytes of
    /// the next response.
    fn return_to_pool(
        &self,
        reader: BufReader<NetworkStream>,
        response: &HttpResponse,
        key: &PoolKey,
    ) {
        if !connection_is_reusable(response) || !reader.buffer().is_empty() {
            return;
        }
        self.pool.checkin(key.clone(), reader.into_inner());
    }

    fn connect(&self, url: &Url, deadline: &Deadline) -> Result<NetworkStream, NetworkError> {
        let addresses = dns::resolve(url.authority())?;
        deadline.check(ProtocolPhase::Connect)?;
        let socket = self.open_socket(&addresses, url, deadline)?;
        if !url.scheme().is_secure() {
            return Ok(NetworkStream::Plain(socket));
        }
        deadline.check(ProtocolPhase::Handshake)?;
        arm_handshake_timeouts(&socket, clamp(self.timeouts.handshake(), deadline))?;
        let session = self.connector.connect(url.host(), socket)?;
        Ok(NetworkStream::Secure(Box::new(session)))
    }

    /// Try every resolved address in order — a host with a stale AAAA record
    /// still connects over IPv4 — and report the last failure if none answers.
    fn open_socket(
        &self,
        addresses: &[SocketAddr],
        url: &Url,
        deadline: &Deadline,
    ) -> Result<TcpStream, NetworkError> {
        let budget = clamp(self.timeouts.connect(), deadline);
        let mut last = NetworkError::unreachable(
            url.authority().to_text(),
            "the resolver returned no usable address",
        );
        for address in addresses {
            match TcpStream::connect_timeout(address, budget) {
                Ok(socket) => return Ok(socket),
                Err(error) => {
                    last = NetworkError::unreachable(url.authority().to_text(), error.to_string());
                }
            }
        }
        Err(last)
    }
}

impl HttpTransport for RealHttpTransport {
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, NetworkError> {
        let deadline = Deadline::starting_now(self.timeouts.total());
        let mut trail = RedirectTrail::new(self.redirects);
        let mut current = request.clone();
        loop {
            deadline.check(ProtocolPhase::Redirect)?;
            trail.record(current.url())?;
            let response = self.exchange(&current, &deadline)?;
            let Some(next) = redirect::next_request(&current, &response)? else {
                return Ok(response);
            };
            current = next;
        }
    }
}

fn write_request(stream: &mut NetworkStream, request: &HttpRequest) -> Result<(), NetworkError> {
    let wire = serialize_request(request);
    stream
        .write_all(&wire)
        .and_then(|()| stream.flush())
        .map_err(|error| NetworkError::transport(ProtocolPhase::Header, error.to_string()))
}

fn arm_handshake_timeouts(socket: &TcpStream, budget: Duration) -> Result<(), NetworkError> {
    socket
        .set_read_timeout(Some(budget))
        .and_then(|()| socket.set_write_timeout(Some(budget)))
        .map_err(|error| NetworkError::transport(ProtocolPhase::Handshake, error.to_string()))
}

/// A phase never gets longer than what is left of the whole exchange, and never
/// gets zero — a zero timeout means "block forever" to the operating system,
/// which is the exact failure this module exists to prevent.
fn clamp(budget: Duration, deadline: &Deadline) -> Duration {
    const FLOOR: Duration = Duration::from_millis(1);
    let remaining = deadline.remaining().unwrap_or(FLOOR);
    budget.min(remaining).max(FLOOR)
}
