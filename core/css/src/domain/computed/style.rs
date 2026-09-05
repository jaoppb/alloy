//! [`ComputedStyle`] — the computed value of every property `core/css`
//! resolves.
//!
//! B0 carried the six of `PRD-007`'s first cut; v0.5 B4 adds everything the
//! real layout engine reads: the third box edge (`border`), the two axes
//! (`width` / `height`), `box-sizing`, the two inline properties (`text-align`,
//! `white-space`), and the nine Flexbox properties — grouped into one
//! [`FlexStyle`] so this aggregate stays readable. The field set is versioned by
//! [`crate::PORT_SCHEMA_VERSION`] and **freezes at I3** (end of B4).

use graphics::Au;

use crate::domain::color::CssColor;
use crate::domain::computed::display::Display;
use crate::domain::computed::edges::LengthEdges;
use crate::domain::computed::flex::FlexStyle;
use crate::domain::computed::inline_style::{TextAlign, WhiteSpace};
use crate::domain::computed::sizing::{BoxSizing, Sizing};
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
    border: LengthEdges,
    padding: LengthEdges,
    font_size: Length,
    width: Sizing,
    height: Sizing,
    box_sizing: BoxSizing,
    text_align: TextAlign,
    white_space: WhiteSpace,
    flex: FlexStyle,
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
            border: LengthEdges::ZERO,
            padding: LengthEdges::ZERO,
            font_size: Length::Pixels(INITIAL_FONT_SIZE_PX),
            width: Sizing::Auto,
            height: Sizing::Auto,
            box_sizing: BoxSizing::ContentBox,
            text_align: TextAlign::Left,
            white_space: WhiteSpace::Normal,
            flex: FlexStyle::initial(),
        }
    }

    /// A fresh style that inherits the inherited properties from `parent` and
    /// takes the `initial` value for the rest.
    ///
    /// The inherited set is exactly CSS's: `color` and `font-size` (CSS Color
    /// L4 / CSS Fonts L4) plus `text-align` and `white-space` (CSS Text L3 §7.3,
    /// §4.1.1). Every box property is **not** inherited, which is why the box
    /// edges, the two axes and the Flexbox group all reset here.
    #[must_use]
    pub const fn inheriting_from(parent: &Self) -> Self {
        Self {
            color: parent.color,
            font_size: parent.font_size,
            text_align: parent.text_align,
            white_space: parent.white_space,
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
    pub const fn with_border(self, border: LengthEdges) -> Self {
        Self { border, ..self }
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
    pub const fn with_width(self, width: Sizing) -> Self {
        Self { width, ..self }
    }

    #[must_use]
    pub const fn with_height(self, height: Sizing) -> Self {
        Self { height, ..self }
    }

    #[must_use]
    pub const fn with_box_sizing(self, box_sizing: BoxSizing) -> Self {
        Self { box_sizing, ..self }
    }

    #[must_use]
    pub const fn with_text_align(self, text_align: TextAlign) -> Self {
        Self { text_align, ..self }
    }

    #[must_use]
    pub const fn with_white_space(self, white_space: WhiteSpace) -> Self {
        Self {
            white_space,
            ..self
        }
    }

    #[must_use]
    pub const fn with_flex(self, flex: FlexStyle) -> Self {
        Self { flex, ..self }
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
    pub const fn border(&self) -> LengthEdges {
        self.border
    }

    #[must_use]
    pub const fn padding(&self) -> LengthEdges {
        self.padding
    }

    #[must_use]
    pub const fn font_size(&self) -> Length {
        self.font_size
    }

    #[must_use]
    pub const fn width(&self) -> Sizing {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> Sizing {
        self.height
    }

    #[must_use]
    pub const fn box_sizing(&self) -> BoxSizing {
        self.box_sizing
    }

    #[must_use]
    pub const fn text_align(&self) -> TextAlign {
        self.text_align
    }

    #[must_use]
    pub const fn white_space(&self) -> WhiteSpace {
        self.white_space
    }

    #[must_use]
    pub const fn flex(&self) -> FlexStyle {
        self.flex
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
