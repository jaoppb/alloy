//! [`LogicalInsets`] — flow-relative position offsets (CSS Logical Properties L1 §6).

use crate::domain::computed::position::PositionStyle;
use crate::domain::computed::sizing::Sizing;

use super::mapping::WritingContext;
use super::side::{LogicalSide, PhysicalSide};

/// Flow-relative offsets: block-start, block-end, inline-start, inline-end.
#[allow(clippy::struct_field_names)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct LogicalInsets {
    block_start: Sizing,
    block_end: Sizing,
    inline_start: Sizing,
    inline_end: Sizing,
}

impl LogicalInsets {
    /// Every offset initialized to `auto`.
    pub const AUTO: Self = Self {
        block_start: Sizing::Auto,
        block_end: Sizing::Auto,
        inline_start: Sizing::Auto,
        inline_end: Sizing::Auto,
    };

    #[must_use]
    pub const fn new(
        block_start: Sizing,
        block_end: Sizing,
        inline_start: Sizing,
        inline_end: Sizing,
    ) -> Self {
        Self {
            block_start,
            block_end,
            inline_start,
            inline_end,
        }
    }

    #[must_use]
    pub const fn block_start(self) -> Sizing {
        self.block_start
    }
    #[must_use]
    pub const fn block_end(self) -> Sizing {
        self.block_end
    }
    #[must_use]
    pub const fn inline_start(self) -> Sizing {
        self.inline_start
    }
    #[must_use]
    pub const fn inline_end(self) -> Sizing {
        self.inline_end
    }

    #[must_use]
    pub const fn with_block_start(self, offset: Sizing) -> Self {
        Self {
            block_start: offset,
            ..self
        }
    }
    #[must_use]
    pub const fn with_block_end(self, offset: Sizing) -> Self {
        Self {
            block_end: offset,
            ..self
        }
    }
    #[must_use]
    pub const fn with_inline_start(self, offset: Sizing) -> Self {
        Self {
            inline_start: offset,
            ..self
        }
    }
    #[must_use]
    pub const fn with_inline_end(self, offset: Sizing) -> Self {
        Self {
            inline_end: offset,
            ..self
        }
    }

    /// These offsets with the one on `side` replaced.
    #[must_use]
    pub const fn with_side(self, side: LogicalSide, value: Sizing) -> Self {
        match side {
            LogicalSide::BlockStart => self.with_block_start(value),
            LogicalSide::BlockEnd => self.with_block_end(value),
            LogicalSide::InlineStart => self.with_inline_start(value),
            LogicalSide::InlineEnd => self.with_inline_end(value),
        }
    }

    /// The physical offset of `position` that `side` maps to in `context` —
    /// the read half of [`Self::apply_to_position`] (CSS Logical L1 §4: the
    /// logical and physical properties share one computed value).
    #[must_use]
    pub const fn read_from_position(
        context: WritingContext,
        position: PositionStyle,
        side: LogicalSide,
    ) -> Sizing {
        match context.map_side(side) {
            PhysicalSide::Top => position.top(),
            PhysicalSide::Right => position.right(),
            PhysicalSide::Bottom => position.bottom(),
            PhysicalSide::Left => position.left(),
        }
    }

    /// Applies a logical inset to the matching physical field on [`PositionStyle`].
    #[must_use]
    pub const fn apply_to_position(
        context: WritingContext,
        position: PositionStyle,
        side: LogicalSide,
        value: Sizing,
    ) -> PositionStyle {
        match context.map_side(side) {
            PhysicalSide::Top => position.with_top(value),
            PhysicalSide::Right => position.with_right(value),
            PhysicalSide::Bottom => position.with_bottom(value),
            PhysicalSide::Left => position.with_left(value),
        }
    }
}
