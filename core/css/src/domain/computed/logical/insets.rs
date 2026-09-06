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
    pub const fn with_block_start(self, val: Sizing) -> Self {
        Self {
            block_start: val,
            ..self
        }
    }
    #[must_use]
    pub const fn with_block_end(self, val: Sizing) -> Self {
        Self {
            block_end: val,
            ..self
        }
    }
    #[must_use]
    pub const fn with_inline_start(self, val: Sizing) -> Self {
        Self {
            inline_start: val,
            ..self
        }
    }
    #[must_use]
    pub const fn with_inline_end(self, val: Sizing) -> Self {
        Self {
            inline_end: val,
            ..self
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
