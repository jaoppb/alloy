//! The nine computed values of the Flexbox module (CSS Flexbox L1), grouped
//! into one [`FlexStyle`] aggregate (v0.5 B4).
//!
//! They are grouped rather than spliced into [`crate::ComputedStyle`] one field
//! at a time for the reason `ADR-0010` rule 7 gives: nine more fields would
//! turn the computed style into a record nobody can read at a glance. The cost
//! is one extra hop (`style.flex().direction()`), the same chain length
//! `node.style().display()` already has.

use core::fmt;

use crate::domain::computed::sizing::Sizing;

/// The direction the main axis runs in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FlexDirection {
    /// Left to right — the `initial` value.
    #[default]
    Row,
    /// Right to left.
    RowReverse,
    /// Top to bottom.
    Column,
    /// Bottom to top.
    ColumnReverse,
}

impl FlexDirection {
    /// Whether the main axis is horizontal.
    #[must_use]
    pub const fn is_horizontal(self) -> bool {
        matches!(self, Self::Row | Self::RowReverse)
    }

    /// Whether items are placed against the far end of the main axis.
    #[must_use]
    pub const fn is_reversed(self) -> bool {
        matches!(self, Self::RowReverse | Self::ColumnReverse)
    }

    /// The keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Row => "row",
            Self::RowReverse => "row-reverse",
            Self::Column => "column",
            Self::ColumnReverse => "column-reverse",
        }
    }
}

impl fmt::Display for FlexDirection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}

/// Whether items overflow the container on one line or break onto several.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FlexWrap {
    /// One line, overflowing if need be — the `initial` value.
    #[default]
    NoWrap,
    /// Several lines, the first one nearest the cross start.
    Wrap,
    /// Several lines, the first one nearest the cross end.
    WrapReverse,
}

impl FlexWrap {
    /// Whether a line may break at all.
    #[must_use]
    pub const fn wraps(self) -> bool {
        matches!(self, Self::Wrap | Self::WrapReverse)
    }

    /// Whether the cross axis is walked backwards.
    #[must_use]
    pub const fn is_reversed(self) -> bool {
        matches!(self, Self::WrapReverse)
    }

    /// The keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::NoWrap => "nowrap",
            Self::Wrap => "wrap",
            Self::WrapReverse => "wrap-reverse",
        }
    }
}

impl fmt::Display for FlexWrap {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}

/// How leftover main-axis space is distributed within a flex line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum JustifyContent {
    /// Packed at the main start — the `initial` value.
    #[default]
    FlexStart,
    /// Packed at the main end.
    FlexEnd,
    /// Packed in the middle.
    Center,
    /// First item at the start, last at the end, gaps equal.
    SpaceBetween,
    /// Equal space around every item — half of it at the two edges.
    SpaceAround,
    /// Equal space between every pair of items and at the two edges.
    SpaceEvenly,
}

impl JustifyContent {
    /// The keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::FlexStart => "flex-start",
            Self::FlexEnd => "flex-end",
            Self::Center => "center",
            Self::SpaceBetween => "space-between",
            Self::SpaceAround => "space-around",
            Self::SpaceEvenly => "space-evenly",
        }
    }
}

impl fmt::Display for JustifyContent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}

/// How an item is placed on the cross axis of its line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AlignItems {
    /// Against the cross start.
    FlexStart,
    /// Against the cross end.
    FlexEnd,
    /// Centred on the cross axis.
    Center,
    /// Filling the line's cross size — the `initial` value.
    #[default]
    Stretch,
    /// Sharing the line's first baseline.
    Baseline,
}

impl AlignItems {
    /// The keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::FlexStart => "flex-start",
            Self::FlexEnd => "flex-end",
            Self::Center => "center",
            Self::Stretch => "stretch",
            Self::Baseline => "baseline",
        }
    }
}

impl fmt::Display for AlignItems {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}

/// How the **lines** of a multi-line container share the cross axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AlignContent {
    /// Packed at the cross start.
    FlexStart,
    /// Packed at the cross end.
    FlexEnd,
    /// Packed in the middle.
    Center,
    /// First line at the start, last at the end, gaps equal.
    SpaceBetween,
    /// Equal space around every line.
    SpaceAround,
    /// Lines share the leftover cross space equally — the `initial` value.
    #[default]
    Stretch,
}

impl AlignContent {
    /// The keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::FlexStart => "flex-start",
            Self::FlexEnd => "flex-end",
            Self::Center => "center",
            Self::SpaceBetween => "space-between",
            Self::SpaceAround => "space-around",
            Self::Stretch => "stretch",
        }
    }
}

impl fmt::Display for AlignContent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}

/// One item's own override of its container's [`AlignItems`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AlignSelf {
    /// Defer to the container — the `initial` value.
    #[default]
    Auto,
    /// Against the cross start.
    FlexStart,
    /// Against the cross end.
    FlexEnd,
    /// Centred on the cross axis.
    Center,
    /// Filling the line's cross size.
    Stretch,
    /// Sharing the line's first baseline.
    Baseline,
}

impl AlignSelf {
    /// This item's effective alignment, given what its container asked for.
    #[must_use]
    pub const fn resolve(self, container: AlignItems) -> AlignItems {
        match self {
            Self::Auto => container,
            Self::FlexStart => AlignItems::FlexStart,
            Self::FlexEnd => AlignItems::FlexEnd,
            Self::Center => AlignItems::Center,
            Self::Stretch => AlignItems::Stretch,
            Self::Baseline => AlignItems::Baseline,
        }
    }

    /// The keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::FlexStart => "flex-start",
            Self::FlexEnd => "flex-end",
            Self::Center => "center",
            Self::Stretch => "stretch",
            Self::Baseline => "baseline",
        }
    }
}

impl fmt::Display for AlignSelf {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}

/// A `flex-grow` / `flex-shrink` factor: a non-negative, finite number.
///
/// The grammar is fractional, so the value object wraps an `f32` — but no
/// geometry is ever computed in `f32`: the layout engine turns a factor into an
/// integer numerator/denominator pair before touching an [`graphics::Au`].
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct FlexFactor(f32);

impl FlexFactor {
    /// The `initial` value of `flex-grow`.
    pub const ZERO: Self = Self(0.0);

    /// The `initial` value of `flex-shrink`.
    pub const ONE: Self = Self(1.0);

    /// A factor, or `None` when the number is negative or non-finite — CSS
    /// Flexbox L1 §7.1.1 makes a negative factor invalid, and there is no
    /// correct reading of `NaN`.
    #[must_use]
    pub fn new(value: f32) -> Option<Self> {
        if !value.is_finite() || value < 0.0 {
            return None;
        }
        Some(Self(value))
    }

    /// The raw factor.
    #[must_use]
    pub const fn value(self) -> f32 {
        self.0
    }

    /// Whether this factor takes no part in distribution.
    #[must_use]
    pub fn is_zero(self) -> bool {
        self.0 == 0.0
    }
}

impl fmt::Display for FlexFactor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// The nine Flexbox properties of one computed style.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FlexStyle {
    direction: FlexDirection,
    wrap: FlexWrap,
    justify_content: JustifyContent,
    align_items: AlignItems,
    align_content: AlignContent,
    align_self: AlignSelf,
    grow: FlexFactor,
    shrink: FlexFactor,
    basis: Sizing,
}

impl FlexStyle {
    /// Every Flexbox property at its CSS `initial` value.
    #[must_use]
    pub const fn initial() -> Self {
        Self {
            direction: FlexDirection::Row,
            wrap: FlexWrap::NoWrap,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Stretch,
            align_content: AlignContent::Stretch,
            align_self: AlignSelf::Auto,
            grow: FlexFactor::ZERO,
            shrink: FlexFactor::ONE,
            basis: Sizing::Auto,
        }
    }

    #[must_use]
    pub const fn with_direction(self, direction: FlexDirection) -> Self {
        Self { direction, ..self }
    }

    #[must_use]
    pub const fn with_wrap(self, wrap: FlexWrap) -> Self {
        Self { wrap, ..self }
    }

    #[must_use]
    pub const fn with_justify_content(self, justify_content: JustifyContent) -> Self {
        Self {
            justify_content,
            ..self
        }
    }

    #[must_use]
    pub const fn with_align_items(self, align_items: AlignItems) -> Self {
        Self {
            align_items,
            ..self
        }
    }

    #[must_use]
    pub const fn with_align_content(self, align_content: AlignContent) -> Self {
        Self {
            align_content,
            ..self
        }
    }

    #[must_use]
    pub const fn with_align_self(self, align_self: AlignSelf) -> Self {
        Self { align_self, ..self }
    }

    #[must_use]
    pub const fn with_grow(self, grow: FlexFactor) -> Self {
        Self { grow, ..self }
    }

    #[must_use]
    pub const fn with_shrink(self, shrink: FlexFactor) -> Self {
        Self { shrink, ..self }
    }

    #[must_use]
    pub const fn with_basis(self, basis: Sizing) -> Self {
        Self { basis, ..self }
    }

    #[must_use]
    pub const fn direction(self) -> FlexDirection {
        self.direction
    }

    #[must_use]
    pub const fn wrap(self) -> FlexWrap {
        self.wrap
    }

    #[must_use]
    pub const fn justify_content(self) -> JustifyContent {
        self.justify_content
    }

    #[must_use]
    pub const fn align_items(self) -> AlignItems {
        self.align_items
    }

    #[must_use]
    pub const fn align_content(self) -> AlignContent {
        self.align_content
    }

    #[must_use]
    pub const fn align_self(self) -> AlignSelf {
        self.align_self
    }

    #[must_use]
    pub const fn grow(self) -> FlexFactor {
        self.grow
    }

    #[must_use]
    pub const fn shrink(self) -> FlexFactor {
        self.shrink
    }

    #[must_use]
    pub const fn basis(self) -> Sizing {
        self.basis
    }
}

impl Default for FlexStyle {
    fn default() -> Self {
        Self::initial()
    }
}
