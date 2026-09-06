//! [`SizingConstraints`] — the minimum and maximum size boundaries
//! (`min-width`, `max-width`, `min-height`, `max-height`).

use crate::domain::computed::sizing::Sizing;

/// Minimum and maximum size boundaries along both axes (CSS Box Sizing L3 §4).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct SizingConstraints {
    min_width: Sizing,
    max_width: Sizing,
    min_height: Sizing,
    max_height: Sizing,
}

impl SizingConstraints {
    /// Every constraint at its CSS `initial` value (`auto` / none).
    #[must_use]
    pub const fn initial() -> Self {
        Self {
            min_width: Sizing::Auto,
            max_width: Sizing::Auto,
            min_height: Sizing::Auto,
            max_height: Sizing::Auto,
        }
    }

    #[must_use]
    pub const fn min_width(self) -> Sizing {
        self.min_width
    }

    #[must_use]
    pub const fn max_width(self) -> Sizing {
        self.max_width
    }

    #[must_use]
    pub const fn min_height(self) -> Sizing {
        self.min_height
    }

    #[must_use]
    pub const fn max_height(self) -> Sizing {
        self.max_height
    }

    #[must_use]
    pub const fn with_min_width(self, min_width: Sizing) -> Self {
        Self { min_width, ..self }
    }

    #[must_use]
    pub const fn with_max_width(self, max_width: Sizing) -> Self {
        Self { max_width, ..self }
    }

    #[must_use]
    pub const fn with_min_height(self, min_height: Sizing) -> Self {
        Self { min_height, ..self }
    }

    #[must_use]
    pub const fn with_max_height(self, max_height: Sizing) -> Self {
        Self { max_height, ..self }
    }
}
