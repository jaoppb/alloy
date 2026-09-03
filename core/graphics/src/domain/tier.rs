//! The rungs of the backend selection cascade (`PRD-005:33-58`).

use core::fmt;

/// Which rendering technology a backend is built on.
///
/// All three rungs exist from v0.3 so that the fall to software is the real
/// algorithm falling through rather than a tautology (**C-17**). `Vulkan` and
/// `OpenGl` are unimplemented until F12 and report themselves unavailable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BackendTier {
    /// `vulkano`, the preferred tier (`ADR-0009`).
    Vulkan,
    /// `glow` / `glutin`, the first fallback.
    OpenGl,
    /// The CPU rasterizer — always available, and the reference every other
    /// tier is compared against.
    Software,
}

impl BackendTier {
    /// The rungs in preference order, highest first.
    pub const CASCADE: [Self; 3] = [Self::Vulkan, Self::OpenGl, Self::Software];

    /// Whether this tier can be relied on to exist on any machine.
    #[must_use]
    pub const fn is_always_available(self) -> bool {
        matches!(self, Self::Software)
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Vulkan => "vulkan",
            Self::OpenGl => "opengl",
            Self::Software => "software",
        }
    }

    /// Parses the value of `GRAPHICS_FORCE_TIER`, the override the cascade tests
    /// use to make each rung fail on demand.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "vulkan" => Some(Self::Vulkan),
            "opengl" => Some(Self::OpenGl),
            "software" => Some(Self::Software),
            _ => None,
        }
    }
}

impl fmt::Display for BackendTier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}
