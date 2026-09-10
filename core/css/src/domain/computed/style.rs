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
use crate::domain::computed::font::FontFamilyList;
use crate::domain::computed::grid::GridStyle;
use crate::domain::computed::inline_style::{TextAlign, WhiteSpace};
use crate::domain::computed::logical::LogicalStyle;
use crate::domain::computed::overflow::OverflowStyle;
use crate::domain::computed::position::PositionStyle;
use crate::domain::computed::sizing::{BoxSizing, Sizing};
use crate::domain::computed::sizing_constraints::SizingConstraints;
use crate::domain::computed::text_advance::TextAdvanceStyle;
use crate::domain::computed::visual::VisualStyle;
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
    font_family: FontFamilyList,
    width: Sizing,
    height: Sizing,
    box_sizing: BoxSizing,
    text_align: TextAlign,
    white_space: WhiteSpace,
    flex: FlexStyle,
    position: PositionStyle,
    constraints: SizingConstraints,
    overflow: OverflowStyle,
    visual: VisualStyle,
    text_advance: TextAdvanceStyle,
    grid: GridStyle,
    logical: LogicalStyle,
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
            font_family: FontFamilyList::empty(),
            width: Sizing::Auto,
            height: Sizing::Auto,
            box_sizing: BoxSizing::ContentBox,
            text_align: TextAlign::Left,
            white_space: WhiteSpace::Normal,
            flex: FlexStyle::initial(),
            position: PositionStyle::initial(),
            constraints: SizingConstraints::initial(),
            overflow: OverflowStyle::initial(),
            visual: VisualStyle::initial(),
            text_advance: TextAdvanceStyle::initial(),
            grid: GridStyle::initial(),
            logical: LogicalStyle::initial(),
        }
    }

    /// A fresh style that inherits the inherited properties from `parent` and
    /// takes the `initial` value for the rest.
    ///
    /// The inherited set is exactly CSS's: `color` and `font-size` /
    /// `font-family` (CSS Color L4 / CSS Fonts L4) plus `text-align` and
    /// `white-space` (CSS Text L3 §7.3, §4.1.1). Every box property is **not**
    /// inherited, which is why the box edges, the two axes and the Flexbox group
    /// all reset here.
    #[must_use]
    pub const fn inheriting_from(parent: &Self) -> Self {
        Self {
            color: parent.color,
            font_size: parent.font_size,
            font_family: parent.font_family,
            text_align: parent.text_align,
            white_space: parent.white_space,
            text_advance: TextAdvanceStyle::inheriting_from(&parent.text_advance),
            logical: parent.logical,
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
    pub const fn with_font_family(self, font_family: FontFamilyList) -> Self {
        Self {
            font_family,
            ..self
        }
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
    pub const fn with_position(self, position: PositionStyle) -> Self {
        Self { position, ..self }
    }

    #[must_use]
    pub const fn with_constraints(self, constraints: SizingConstraints) -> Self {
        Self {
            constraints,
            ..self
        }
    }

    #[must_use]
    pub const fn with_overflow(self, overflow: OverflowStyle) -> Self {
        Self { overflow, ..self }
    }

    #[must_use]
    pub const fn with_visual(self, visual: VisualStyle) -> Self {
        Self { visual, ..self }
    }

    #[must_use]
    pub const fn with_text_advance(self, text_advance: TextAdvanceStyle) -> Self {
        Self {
            text_advance,
            ..self
        }
    }

    #[must_use]
    pub const fn with_grid(self, grid: GridStyle) -> Self {
        Self { grid, ..self }
    }

    #[must_use]
    pub const fn with_logical(self, logical: LogicalStyle) -> Self {
        Self { logical, ..self }
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
    pub const fn font_family(&self) -> FontFamilyList {
        self.font_family
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

    #[must_use]
    pub const fn position(&self) -> PositionStyle {
        self.position
    }

    #[must_use]
    pub const fn constraints(&self) -> SizingConstraints {
        self.constraints
    }

    #[must_use]
    pub const fn overflow(&self) -> OverflowStyle {
        self.overflow
    }

    #[must_use]
    pub const fn visual(&self) -> VisualStyle {
        self.visual
    }

    #[must_use]
    pub const fn text_advance(&self) -> TextAdvanceStyle {
        self.text_advance
    }

    #[must_use]
    pub const fn grid(&self) -> GridStyle {
        self.grid
    }

    #[must_use]
    pub const fn logical(&self) -> LogicalStyle {
        self.logical
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
