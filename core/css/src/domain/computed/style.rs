//! [`ComputedStyle`] — the computed value of every property `core/css` resolves
//! in B0.
//!
//! Exactly the six of `crate::SUPPORTED_PROPERTIES`: `display`, `color`,
//! `background-color`, `margin`, `padding`, `font-size`. B2 widens this as the
//! real cascade lands; the field set is versioned by
//! [`crate::PORT_SCHEMA_VERSION`] and freezes at I3.

use graphics::Au;

use crate::domain::color::CssColor;
use crate::domain::computed::display::Display;
use crate::domain::computed::edges::LengthEdges;
use crate::domain::length::Length;

/// The CSS `initial` computed `font-size`: `16px`.
const INITIAL_FONT_SIZE_PX: f32 = 16.0;

/// A node's fully-resolved style, ready for layout.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ComputedStyle {
    display: Display,
    color: CssColor,
    background_color: CssColor,
    margin: LengthEdges,
    padding: LengthEdges,
    font_size: Length,
}

impl ComputedStyle {
    /// Every property at its CSS `initial` value.
    #[must_use]
    pub const fn initial() -> Self {
        Self {
            display: Display::Block,
            color: CssColor::BLACK,
            background_color: CssColor::TRANSPARENT,
            margin: LengthEdges::ZERO,
            padding: LengthEdges::ZERO,
            font_size: Length::Pixels(INITIAL_FONT_SIZE_PX),
        }
    }

    /// A fresh style that inherits the inherited properties from `parent`
    /// (`color`, `font-size`) and takes the `initial` value for the rest.
    ///
    /// The only two inherited properties B0 tracks are the ones the placeholder
    /// [`crate::BlockLayout`] and painter read.
    #[must_use]
    pub const fn inheriting_from(parent: &Self) -> Self {
        Self {
            color: parent.color,
            font_size: parent.font_size,
            ..Self::initial()
        }
    }

    #[must_use]
    pub const fn with_display(self, display: Display) -> Self {
        Self { display, ..self }
    }

    #[must_use]
    pub const fn with_color(self, color: CssColor) -> Self {
        Self { color, ..self }
    }

    #[must_use]
    pub const fn with_background_color(self, background_color: CssColor) -> Self {
        Self {
            background_color,
            ..self
        }
    }

    #[must_use]
    pub const fn with_margin(self, margin: LengthEdges) -> Self {
        Self { margin, ..self }
    }

    #[must_use]
    pub const fn with_padding(self, padding: LengthEdges) -> Self {
        Self { padding, ..self }
    }

    #[must_use]
    pub const fn with_font_size(self, font_size: Length) -> Self {
        Self { font_size, ..self }
    }

    #[must_use]
    pub const fn display(&self) -> Display {
        self.display
    }

    #[must_use]
    pub const fn color(&self) -> CssColor {
        self.color
    }

    #[must_use]
    pub const fn background_color(&self) -> CssColor {
        self.background_color
    }

    #[must_use]
    pub const fn margin(&self) -> LengthEdges {
        self.margin
    }

    #[must_use]
    pub const fn padding(&self) -> LengthEdges {
        self.padding
    }

    #[must_use]
    pub const fn font_size(&self) -> Length {
        self.font_size
    }

    /// The computed `font-size` resolved to a computed length, for layout and
    /// text measurement. `em`/`%` in `font-size` itself resolve against
    /// `parent_font_size` (the CSS rule for the property).
    #[must_use]
    pub fn font_size_au(&self, parent_font_size: Au) -> Option<Au> {
        self.font_size
            .resolve_to_au(parent_font_size, parent_font_size)
    }
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self::initial()
    }
}
