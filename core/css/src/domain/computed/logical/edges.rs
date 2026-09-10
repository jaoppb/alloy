//! [`LogicalEdges`] — flow-relative box edges (margin, padding, border-width)
//! along the block and inline dimensions (CSS Logical Properties L1 §4).

use crate::domain::computed::edges::LengthEdges;
use crate::domain::length::Length;

use super::mapping::WritingContext;
use super::side::{LogicalSide, PhysicalSide};

/// The four logical sides of a box property: block-start, inline-end, block-end, inline-start.
#[allow(clippy::struct_field_names)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct LogicalEdges {
    block_start: Length,
    inline_end: Length,
    block_end: Length,
    inline_start: Length,
}

impl LogicalEdges {
    /// All four logical sides zero.
    pub const ZERO: Self = Self {
        block_start: Length::ZERO,
        inline_end: Length::ZERO,
        block_end: Length::ZERO,
        inline_start: Length::ZERO,
    };

    /// Distinct lengths per logical side.
    #[must_use]
    pub const fn new(
        block_start: Length,
        inline_end: Length,
        block_end: Length,
        inline_start: Length,
    ) -> Self {
        Self {
            block_start,
            inline_end,
            block_end,
            inline_start,
        }
    }

    /// The same length on every logical side.
    #[must_use]
    pub const fn uniform(length: Length) -> Self {
        Self::new(length, length, length, length)
    }

    /// Axial logical edges: block axis sides equal, inline axis sides equal.
    #[must_use]
    pub const fn from_axes(block: Length, inline: Length) -> Self {
        Self::new(block, inline, block, inline)
    }

    #[must_use]
    pub const fn block_start(self) -> Length {
        self.block_start
    }
    #[must_use]
    pub const fn inline_end(self) -> Length {
        self.inline_end
    }
    #[must_use]
    pub const fn block_end(self) -> Length {
        self.block_end
    }
    #[must_use]
    pub const fn inline_start(self) -> Length {
        self.inline_start
    }

    #[must_use]
    pub const fn with_block_start(self, length: Length) -> Self {
        Self {
            block_start: length,
            ..self
        }
    }
    #[must_use]
    pub const fn with_block_end(self, length: Length) -> Self {
        Self {
            block_end: length,
            ..self
        }
    }
    #[must_use]
    pub const fn with_inline_start(self, length: Length) -> Self {
        Self {
            inline_start: length,
            ..self
        }
    }
    #[must_use]
    pub const fn with_inline_end(self, length: Length) -> Self {
        Self {
            inline_end: length,
            ..self
        }
    }

    /// Updates the single physical side that corresponds to `side` in `context`.
    #[must_use]
    pub const fn apply_to_physical(
        context: WritingContext,
        edges: LengthEdges,
        side: LogicalSide,
        value: Length,
    ) -> LengthEdges {
        match context.map_side(side) {
            PhysicalSide::Top => edges.with_top(value),
            PhysicalSide::Right => edges.with_right(value),
            PhysicalSide::Bottom => edges.with_bottom(value),
            PhysicalSide::Left => edges.with_left(value),
        }
    }

    /// Converts this entire logical quartet to physical [`LengthEdges`].
    #[must_use]
    pub const fn to_physical(self, context: WritingContext) -> LengthEdges {
        let e1 = Self::apply_to_physical(
            context,
            LengthEdges::ZERO,
            LogicalSide::BlockStart,
            self.block_start,
        );
        let e2 = Self::apply_to_physical(context, e1, LogicalSide::BlockEnd, self.block_end);
        let e3 = Self::apply_to_physical(context, e2, LogicalSide::InlineStart, self.inline_start);
        Self::apply_to_physical(context, e3, LogicalSide::InlineEnd, self.inline_end)
    }
}
