//! The three-tier selection cascade of `PRD-005:33-58` — **C-17**.
//!
//! ## Why all three rungs exist in v0.3
//!
//! It would be simpler to return `SoftwareCpuBackend::new()` and call the
//! fallback "done". That would make **C-17** true by tautology rather than by
//! construction: nothing would have fallen, because nothing would have been
//! attempted. So `vulkan.rs` and `opengl.rs` exist, report
//! [`GraphicsError::BackendUnavailable`], and the cascade genuinely walks past
//! them. When F12 implements them, this file does not change.
//!
//! ## Why failures are returned rather than logged
//!
//! `PRD-005:33-58` asks for a warning to `DevTools` when a rung is skipped.
//! Emitting it here would make this crate depend on a logging facade and decide
//! a *policy* question — how loudly to complain — that belongs to whoever
//! composes the browser (`ADR-0003`). [`BackendSelection`] therefore carries the
//! rungs that were skipped and why, and `alloy` logs them through `tracing`
//! (`ADR-0014`).
//!
//! ## Testing the fall without touching the process
//!
//! [`select_backend_with`] takes the per-rung constructor as an argument, so a
//! test makes a rung fail by passing a closure — no environment variable, no
//! shared mutable state, no interference between tests running in parallel.
//! `GRAPHICS_FORCE_TIER` exists for operators, and is read only by
//! [`BackendPreference::from_environment`].

use crate::application::ports::RenderBackend;
use crate::domain::error::GraphicsError;
use crate::domain::tier::BackendTier;

/// The environment variable an operator uses to pin the starting rung.
pub const FORCE_TIER_VARIABLE: &str = "GRAPHICS_FORCE_TIER";

/// Which rungs a caller is willing to accept.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum BackendPreference {
    /// Walk the cascade from the top, taking the best rung that initialises.
    #[default]
    Automatic,
    /// Skip everything above `tier`, then walk down from there. A headless CI
    /// sets this to [`BackendTier::Software`] to avoid probing GPUs at all.
    StartingAt(BackendTier),
}

impl BackendPreference {
    /// Reads [`FORCE_TIER_VARIABLE`], falling back to
    /// [`BackendPreference::Automatic`] when it is absent or unrecognised.
    ///
    /// An unrecognised value is *ignored* rather than refused: a typo in an
    /// environment variable should not stop a browser from starting.
    #[must_use]
    pub fn from_environment() -> Self {
        let Ok(raw) = std::env::var(FORCE_TIER_VARIABLE) else {
            return Self::Automatic;
        };
        BackendTier::parse(&raw).map_or(Self::Automatic, Self::StartingAt)
    }

    /// Whether `tier` is at or below the requested starting point.
    const fn admits(self, tier: BackendTier) -> bool {
        match self {
            Self::Automatic => true,
            Self::StartingAt(start) => tier.rank() >= start.rank(),
        }
    }
}

/// The chosen backend, plus the rungs that were tried and refused.
///
/// The skipped list is the diagnostic `PRD-005:33-58` asks for. It is data, so
/// the composer decides whether it is a warning, a `DevTools` event, or nothing.
pub struct BackendSelection {
    backend: Box<dyn RenderBackend>,
    skipped: Vec<(BackendTier, GraphicsError)>,
}

impl BackendSelection {
    /// The backend to render with.
    #[must_use]
    pub fn into_backend(self) -> Box<dyn RenderBackend> {
        self.backend
    }

    /// Which tier was selected.
    #[must_use]
    pub fn tier(&self) -> BackendTier {
        self.backend.tier()
    }

    /// The rungs that were attempted and failed, in the order they were tried.
    #[must_use]
    pub fn skipped(&self) -> &[(BackendTier, GraphicsError)] {
        &self.skipped
    }
}

impl core::fmt::Debug for BackendSelection {
    /// Hand-written because `Box<dyn RenderBackend>` cannot derive `Debug` — and
    /// the useful content is the outcome anyway: which rung answered, and which
    /// ones refused first.
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("BackendSelection")
            .field("tier", &self.tier())
            .field("skipped", &self.skipped)
            // `finish_non_exhaustive` rather than `finish`: `backend` is a
            // `Box<dyn RenderBackend>` with nothing printable beyond the tier
            // already shown above.
            .finish_non_exhaustive()
    }
}

/// Walks the cascade with the real per-rung constructors.
pub fn select_backend(preference: BackendPreference) -> Result<BackendSelection, GraphicsError> {
    select_backend_with(preference, &initialise_tier)
}

/// Walks the cascade with `attempt` standing in for the rung constructors.
///
/// This is the whole algorithm; [`select_backend`] is one call to it. Injecting
/// `attempt` is what lets a test exercise every fall deterministically.
pub fn select_backend_with(
    preference: BackendPreference,
    attempt: &dyn Fn(BackendTier) -> Result<Box<dyn RenderBackend>, GraphicsError>,
) -> Result<BackendSelection, GraphicsError> {
    let mut skipped = Vec::new();
    for tier in BackendTier::CASCADE {
        if !preference.admits(tier) {
            continue;
        }
        match attempt(tier) {
            Ok(backend) => return Ok(BackendSelection { backend, skipped }),
            Err(refusal) => skipped.push((tier, refusal)),
        }
    }
    Err(exhausted(&skipped))
}

/// The error for a cascade in which every admitted rung refused.
///
/// Reports the *last* rung tried, because that is the one whose absence is
/// surprising: software is always available in any build that links it, so
/// reaching here means either `no-backend` or a deliberately narrowed
/// preference.
fn exhausted(skipped: &[(BackendTier, GraphicsError)]) -> GraphicsError {
    let tier = skipped
        .last()
        .map_or(BackendTier::Software, |(tier, _)| *tier);
    GraphicsError::BackendUnavailable { tier }
}

/// Brings up one rung.
fn initialise_tier(tier: BackendTier) -> Result<Box<dyn RenderBackend>, GraphicsError> {
    match tier {
        BackendTier::Vulkan => super::vulkan::initialise(),
        BackendTier::OpenGl => super::opengl::initialise(),
        BackendTier::Software => initialise_software(),
    }
}

/// The software rung, which is present only when it is linked.
// Infallible in this configuration, fallible in the `no-backend` one. Both must
// share a signature for `initialise_tier` to stay a plain `match`, and the rung
// contract is "attempt, and report a refusal" — so the `Result` is the contract,
// not an accident of this branch.
#[allow(clippy::unnecessary_wraps)]
#[cfg(feature = "software-backend")]
fn initialise_software() -> Result<Box<dyn RenderBackend>, GraphicsError> {
    Ok(Box::new(super::software::SoftwareCpuBackend::new()))
}

/// Under `no-backend` there is no rasterizer at all, so even the last rung
/// refuses — and `select_backend` reports it instead of pretending.
#[cfg(not(feature = "software-backend"))]
fn initialise_software() -> Result<Box<dyn RenderBackend>, GraphicsError> {
    Err(GraphicsError::BackendUnavailable {
        tier: BackendTier::Software,
    })
}
