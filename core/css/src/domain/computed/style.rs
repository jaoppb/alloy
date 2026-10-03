//! [`ComputedStyle`] — the computed value of every property `core/css`
//! resolves.
//!
//! B0 carried the six of `PRD-007`'s first cut; v0.5 B4 adds everything the
//! real layout engine reads: the third box edge (`border`), the two axes
//! (`width` / `height`), `box-sizing`, the two inline properties (`text-align`,
//! `white-space`), and the nine Flexbox properties — grouped into one
//! [`FlexStyle`] so this aggregate stays readable. The field set is versioned by
//! [`crate::PORT_SCHEMA_VERSION`] and **freezes at I3** (end of B4).
//!
//! Schema 8 made the aggregate `Clone` but no longer `Copy`: the CSS Grid group
//! ([`GridStyle`], ~2.9 KB of fixed-capacity track lists, area names and line
//! names) lives behind a shared [`Arc`] that stays `None` while every grid
//! property is at its `initial` value — which is every node of a page with no
//! grid. Copying ~3.7 KB through every cascade step and storing it per
//! [`crate::StyledNode`] was the cost of keeping data no layout reads yet
//! (`display: grid` is rejected) inline in the hot aggregate.

use std::sync::Arc;

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

/// [`INITIAL_FONT_SIZE_PX`] as an [`Au`] — the size the root element's `em`
/// resolves against, since it has no parent.
pub const INITIAL_FONT_SIZE: Au = match Au::from_whole_px(16) {
    Some(size) => size,
    None => Au::ZERO,
};

/// The value [`ComputedStyle::grid`] answers while the grid group is at its
/// `initial` value. A `static`, not a `const`, so every such answer borrows the
/// one same value instead of a fresh promoted temporary.
static INITIAL_GRID: GridStyle = GridStyle::initial();

/// A node's fully-resolved style, ready for layout.
///
/// `Clone` but not `Copy` since schema 8 (module doc): cloning bumps the grid
/// group's reference count instead of copying it.
#[derive(Clone, Debug, PartialEq)]
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
    /// `None` while every grid property is `initial` — the invariant
    /// [`Self::with_grid`] keeps, so the derived `PartialEq` never has to
    /// tell a `None` from a `Some` holding the initial value.
    grid: Option<Arc<GridStyle>>,
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
            grid: None,
            logical: LogicalStyle::initial(),
        }
    }

    /// A fresh style that inherits the inherited properties from `parent` and
    /// takes the `initial` value for the rest.
    ///
    /// The inherited set is exactly CSS's: `color` and `font-size` /
    /// `font-family` (CSS Color L4 / CSS Fonts L4) plus `text-align` and
    /// `white-space` (CSS Text L3 §7.3, §4.1.1), plus `writing-mode` and
    /// `direction` (CSS Writing Modes L3 §3.1, §2.1) — the writing context.
    /// Every box property is **not** inherited, which is why the box edges, the
    /// two axes, the Flexbox group and the flow-relative margins, paddings,
    /// borders, insets and sizes of [`LogicalStyle`] all reset here.
    #[must_use]
    pub fn inheriting_from(parent: &Self) -> Self {
        Self {
            color: parent.color,
            font_size: parent.font_size,
            font_family: parent.font_family,
            text_align: parent.text_align,
            white_space: parent.white_space,
            text_advance: TextAdvanceStyle::inheriting_from(&parent.text_advance),
            logical: LogicalStyle::initial().with_context(parent.logical.context()),
            ..Self::initial()
        }
    }

    #[must_use]
    pub fn with_display(self, display: Display) -> Self {
        Self { display, ..self }
    }

    #[must_use]
    pub fn with_color(self, color: CssColor) -> Self {
        Self { color, ..self }
    }

    #[must_use]
    pub fn with_background_color(self, background_color: CssColor) -> Self {
        Self {
            background_color,
            ..self
        }
    }

    #[must_use]
    pub fn with_margin(self, margin: LengthEdges) -> Self {
        Self { margin, ..self }
    }

    #[must_use]
    pub fn with_border(self, border: LengthEdges) -> Self {
        Self { border, ..self }
    }

    #[must_use]
    pub fn with_padding(self, padding: LengthEdges) -> Self {
        Self { padding, ..self }
    }

    #[must_use]
    pub fn with_font_size(self, font_size: Length) -> Self {
        Self { font_size, ..self }
    }

    #[must_use]
    pub fn with_font_family(self, font_family: FontFamilyList) -> Self {
        Self {
            font_family,
            ..self
        }
    }

    #[must_use]
    pub fn with_width(self, width: Sizing) -> Self {
        Self { width, ..self }
    }

    #[must_use]
    pub fn with_height(self, height: Sizing) -> Self {
        Self { height, ..self }
    }

    #[must_use]
    pub fn with_box_sizing(self, box_sizing: BoxSizing) -> Self {
        Self { box_sizing, ..self }
    }

    #[must_use]
    pub fn with_text_align(self, text_align: TextAlign) -> Self {
        Self { text_align, ..self }
    }

    #[must_use]
    pub fn with_white_space(self, white_space: WhiteSpace) -> Self {
        Self {
            white_space,
            ..self
        }
    }

    #[must_use]
    pub fn with_flex(self, flex: FlexStyle) -> Self {
        Self { flex, ..self }
    }

    #[must_use]
    pub fn with_position(self, position: PositionStyle) -> Self {
        Self { position, ..self }
    }

    #[must_use]
    pub fn with_constraints(self, constraints: SizingConstraints) -> Self {
        Self {
            constraints,
            ..self
        }
    }

    #[must_use]
    pub fn with_overflow(self, overflow: OverflowStyle) -> Self {
        Self { overflow, ..self }
    }

    #[must_use]
    pub fn with_visual(self, visual: VisualStyle) -> Self {
        Self { visual, ..self }
    }

    #[must_use]
    pub fn with_text_advance(self, text_advance: TextAdvanceStyle) -> Self {
        Self {
            text_advance,
            ..self
        }
    }

    /// `self` with the grid group replaced. The `initial` group is stored as
    /// `None`, so a style that never sets a grid property allocates nothing.
    #[must_use]
    pub fn with_grid(self, grid: GridStyle) -> Self {
        let shared = (grid != GridStyle::initial()).then(|| Arc::new(grid));
        Self {
            grid: shared,
            ..self
        }
    }

    #[must_use]
    pub fn with_logical(self, logical: LogicalStyle) -> Self {
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

    /// The grid group — borrowed, because it is the one group too large to
    /// hand out by value (module doc).
    #[must_use]
    pub fn grid(&self) -> &GridStyle {
        self.grid.as_deref().unwrap_or(&INITIAL_GRID)
    }

    #[must_use]
    pub const fn logical(&self) -> LogicalStyle {
        self.logical
    }

    /// The computed `font-size` in [`Au`], resolved against the parent's
    /// computed size, or [`INITIAL_FONT_SIZE`] when the author wrote a
    /// magnitude with no correct reading — the same rule layout applies
    /// (`layout::box_model::font_size_of`), so the cascade absolutizes
    /// font-relative lengths against exactly the size layout will use.
    #[must_use]
    pub fn computed_font_size(&self, parent_font_size: Au) -> Au {
        self.font_size_au(parent_font_size)
            .unwrap_or(INITIAL_FONT_SIZE)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::computed::grid::GridGap;

    fn non_initial_grid() -> GridStyle {
        GridStyle::initial().with_gap(GridGap::uniform(Length::Pixels(8.0)))
    }

    #[test]
    fn the_initial_grid_is_stored_as_nothing() {
        assert!(ComputedStyle::initial().grid.is_none());
        let reset = ComputedStyle::initial().with_grid(GridStyle::initial());
        assert!(reset.grid.is_none());
        let cleared = ComputedStyle::initial()
            .with_grid(non_initial_grid())
            .with_grid(GridStyle::initial());
        assert!(cleared.grid.is_none());
        assert_eq!(cleared, ComputedStyle::initial());
    }

    #[test]
    fn a_non_initial_grid_is_shared_between_clones() {
        let style = ComputedStyle::initial().with_grid(non_initial_grid());
        let clone = style.clone();
        let (Some(original), Some(cloned)) = (&style.grid, &clone.grid) else {
            panic!("a non-initial grid is stored");
        };
        assert!(Arc::ptr_eq(original, cloned));
        assert_eq!(clone.grid(), &non_initial_grid());
    }
}
