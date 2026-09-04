//! A bounded `keep-alive` connection pool, keyed by origin.
//!
//! Reuse is an optimisation and must never be observable: a response served
//! over a pooled socket is byte-identical to one served over a fresh socket,
//! and a pooled socket the peer has since closed causes a transparent
//! reconnect rather than a failure. That is what the re-entrancy rule of
//! [`run_transport_suite`](crate::application::conformance::run_transport_suite)
//! checks.
//!
//! Bounded on purpose: an unbounded pool is a file-descriptor leak with extra
//! steps.

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::domain::authority::Authority;
use crate::domain::scheme::Scheme;
use crate::domain::url::Url;
use crate::infrastructure::stream::NetworkStream;

/// How many idle connections one origin may keep.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PoolCapacity(usize);

impl PoolCapacity {
    /// Six per origin — the figure HTTP/1.1 clients have converged on.
    pub const DEFAULT: Self = Self(6);

    /// A capacity of `connections` idle connections per origin.
    #[must_use]
    pub const fn of_connections(connections: usize) -> Self {
        Self(connections)
    }

    /// The capacity.
    #[must_use]
    pub const fn connections(self) -> usize {
        self.0
    }
}

impl Default for PoolCapacity {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// What connections are bucketed by: scheme, host and port together.
///
/// The scheme is part of the key because a cleartext and a TLS connection to
/// the same host and port are not interchangeable.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PoolKey {
    scheme: Scheme,
    authority: Authority,
}

impl PoolKey {
    /// The key a URL belongs under.
    #[must_use]
    pub fn of(url: &Url) -> Self {
        Self {
            scheme: url.scheme(),
            authority: url.authority().clone(),
        }
    }
}

/// Idle connections, held until they are wanted again.
#[derive(Debug, Default)]
pub struct ConnectionPool {
    idle: Mutex<BTreeMap<PoolKey, Vec<NetworkStream>>>,
    capacity: PoolCapacity,
}

impl ConnectionPool {
    /// A pool with the default per-origin capacity.
    #[must_use]
    pub const fn new() -> Self {
        Self::with_capacity(PoolCapacity::DEFAULT)
    }

    /// A pool with an explicit per-origin capacity.
    #[must_use]
    pub const fn with_capacity(capacity: PoolCapacity) -> Self {
        Self {
            idle: Mutex::new(BTreeMap::new()),
            capacity,
        }
    }

    /// Take an idle connection for `key`, if there is one.
    pub fn checkout(&self, key: &PoolKey) -> Option<NetworkStream> {
        let mut idle = self.locked();
        idle.get_mut(key).and_then(Vec::pop)
    }

    /// Return a connection for reuse, dropping it when the bucket is full.
    // The lock guard spans `entry` + `push` because both must be one critical
    // section; `capacity` is read before the lock. `significant_drop_tightening`
    // would have us split a correct minimal section — same "lint fights a
    // deliberate mutex idiom" carve-out `arch-lint.toml` documents for
    // `no-silent-result-drop`.
    #[allow(clippy::significant_drop_tightening)]
    pub fn checkin(&self, key: PoolKey, connection: NetworkStream) {
        let capacity = self.capacity.connections();
        let mut idle = self.locked();
        let bucket = idle.entry(key).or_default();
        if bucket.len() < capacity {
            bucket.push(connection);
        }
    }

    /// How many idle connections are held, across every origin.
    #[must_use]
    pub fn idle_count(&self) -> usize {
        let idle = self.locked();
        idle.values().map(Vec::len).sum()
    }

    /// `x.lock().unwrap_or_else(|poison| poison.into_inner())` is this
    /// workspace's mutex-poison idiom (`arch-lint.toml`, `no-silent-result-drop`
    /// disabled): a thread that panicked while holding the pool lock must not
    /// wedge every later request. The map's invariants do not depend on the
    /// panicking section having finished.
    fn locked(&self) -> std::sync::MutexGuard<'_, BTreeMap<PoolKey, Vec<NetworkStream>>> {
        self.idle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
