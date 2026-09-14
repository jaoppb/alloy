//! [`ViewportConstraints`] — the sizing input to [`crate::LayoutEngine::layout`].
//!
//! Width and height in computed [`Au`] (`ADR-0016`), so layout arithmetic is
//! integer arithmetic and the 100-run determinism gate of `PRD-007:100` holds.

use graphics::Au;

/// The available space a layout is performed into.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ViewportConstraints {
    width: Au,
    height: Au,
}

impl ViewportConstraints {
    #[must_use]
    pub const fn new(width: Au, height: Au) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn width(self) -> Au {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> Au {
        self.height
    }
}
