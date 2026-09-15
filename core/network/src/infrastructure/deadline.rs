//! [`Deadline`] and [`PhaseTimeouts`] — why a hostile server cannot make this
//! client hang.
//!
//! A socket read timeout alone is not enough: a peer that dribbles one byte
//! every half second never trips a one-second read timeout and still holds the
//! connection open for hours. So every read loop also checks a wall-clock
//! deadline for the whole exchange, and gives up with
//! [`NetworkError::Timeout`] naming the phase it was in.
//!
//! Pure `std::time`: no socket, so this module is not gated behind
//! `real-transport` and the ungated readers can enforce a deadline too.

use std::time::{Duration, Instant};

use crate::domain::error::NetworkError;
use crate::domain::phase::ProtocolPhase;

/// The per-phase budgets one exchange runs under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PhaseTimeouts {
    connect: Duration,
    handshake: Duration,
    header: Duration,
    body: Duration,
    total: Duration,
}

impl PhaseTimeouts {
    /// The defaults an interactive browser wants.
    ///
    /// There is deliberately no DNS budget: `std::net::ToSocketAddrs` offers no
    /// way to bound itself, which `ADR-0019` answers by running the whole
    /// `execute` on a pool worker the consumer can abandon. This is recorded
    /// here rather than left as folklore.
    pub const DEFAULT: Self = Self {
        connect: Duration::from_secs(10),
        handshake: Duration::from_secs(10),
        header: Duration::from_secs(15),
        body: Duration::from_secs(30),
        total: Duration::from_mins(1),
    };

    /// Budgets sized for a test against a loopback server: everything is
    /// milliseconds away, so a hostile fixture fails fast instead of holding
    /// the suite for a minute.
    pub const FAST: Self = Self {
        connect: Duration::from_secs(2),
        handshake: Duration::from_secs(2),
        header: Duration::from_secs(2),
        body: Duration::from_secs(2),
        total: Duration::from_secs(5),
    };

    /// The TCP connect budget.
    #[must_use]
    pub const fn connect(self) -> Duration {
        self.connect
    }

    /// The TLS handshake budget.
    #[must_use]
    pub const fn handshake(self) -> Duration {
        self.handshake
    }

    /// The status-line-and-fields budget.
    #[must_use]
    pub const fn header(self) -> Duration {
        self.header
    }

    /// The body budget.
    #[must_use]
    pub const fn body(self) -> Duration {
        self.body
    }

    /// The whole-exchange budget, redirects included.
    #[must_use]
    pub const fn total(self) -> Duration {
        self.total
    }
}

impl Default for PhaseTimeouts {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A wall-clock instant an operation must finish before.
#[derive(Clone, Copy, Debug)]
pub struct Deadline {
    started_at: Instant,
    budget: Duration,
}

impl Deadline {
    /// A deadline `budget` from now.
    #[must_use]
    pub fn starting_now(budget: Duration) -> Self {
        Self {
            started_at: Instant::now(),
            budget,
        }
    }

    /// How long is left, or `None` when the budget is spent.
    #[must_use]
    pub fn remaining(&self) -> Option<Duration> {
        self.budget.checked_sub(self.started_at.elapsed())
    }

    /// How long this deadline has been running, in milliseconds — what the
    /// [`NetworkError::Timeout`] variant reports.
    #[must_use]
    pub fn elapsed_millis(&self) -> u64 {
        u64::try_from(self.started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Fail if the budget is spent.
    ///
    /// # Errors
    ///
    /// [`NetworkError::Timeout`] carrying `phase` and the elapsed time.
    pub fn check(&self, phase: ProtocolPhase) -> Result<(), NetworkError> {
        if self.remaining().is_some() {
            return Ok(());
        }
        Err(NetworkError::timeout(phase, self.elapsed_millis()))
    }
}
