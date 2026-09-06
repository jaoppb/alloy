//! Logical-to-physical axis and side mapping (CSS Logical Properties L1 §2).

use super::axis::{LogicalAxis, PhysicalAxis};
use super::direction::Direction;
use super::side::{LogicalSide, PhysicalSide};
use super::writing_mode::WritingMode;

/// Context capturing the writing mode and direction for resolving flow-relative coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct WritingContext {
    writing_mode: WritingMode,
    direction: Direction,
}

impl WritingContext {
    /// Creates a writing context with the given mode and direction.
    #[must_use]
    pub const fn new(writing_mode: WritingMode, direction: Direction) -> Self {
        Self {
            writing_mode,
            direction,
        }
    }

    /// The active writing mode.
    #[must_use]
    pub const fn writing_mode(self) -> WritingMode {
        self.writing_mode
    }

    /// The active direction.
    #[must_use]
    pub const fn direction(self) -> Direction {
        self.direction
    }

    /// Resolves a logical axis to its physical counterpart.
    #[must_use]
    pub const fn map_axis(self, axis: LogicalAxis) -> PhysicalAxis {
        match (self.writing_mode, axis) {
            (WritingMode::HorizontalTb, LogicalAxis::Block)
            | (WritingMode::VerticalRl | WritingMode::VerticalLr, LogicalAxis::Inline) => {
                PhysicalAxis::Vertical
            }
            (WritingMode::HorizontalTb, LogicalAxis::Inline)
            | (WritingMode::VerticalRl | WritingMode::VerticalLr, LogicalAxis::Block) => {
                PhysicalAxis::Horizontal
            }
        }
    }

    /// Resolves a logical side to its physical box side (CSS Logical Properties L1 §2.3).
    #[must_use]
    pub const fn map_side(self, side: LogicalSide) -> PhysicalSide {
        match (self.writing_mode, self.direction, side) {
            (WritingMode::HorizontalTb, _, LogicalSide::BlockStart)
            | (
                WritingMode::VerticalRl | WritingMode::VerticalLr,
                Direction::Ltr,
                LogicalSide::InlineStart,
            )
            | (
                WritingMode::VerticalRl | WritingMode::VerticalLr,
                Direction::Rtl,
                LogicalSide::InlineEnd,
            ) => PhysicalSide::Top,

            (WritingMode::HorizontalTb, _, LogicalSide::BlockEnd)
            | (
                WritingMode::VerticalRl | WritingMode::VerticalLr,
                Direction::Ltr,
                LogicalSide::InlineEnd,
            )
            | (
                WritingMode::VerticalRl | WritingMode::VerticalLr,
                Direction::Rtl,
                LogicalSide::InlineStart,
            ) => PhysicalSide::Bottom,

            (WritingMode::HorizontalTb, Direction::Ltr, LogicalSide::InlineStart)
            | (WritingMode::HorizontalTb, Direction::Rtl, LogicalSide::InlineEnd)
            | (WritingMode::VerticalRl, _, LogicalSide::BlockEnd)
            | (WritingMode::VerticalLr, _, LogicalSide::BlockStart) => PhysicalSide::Left,

            (WritingMode::HorizontalTb, Direction::Ltr, LogicalSide::InlineEnd)
            | (WritingMode::HorizontalTb, Direction::Rtl, LogicalSide::InlineStart)
            | (WritingMode::VerticalRl, _, LogicalSide::BlockStart)
            | (WritingMode::VerticalLr, _, LogicalSide::BlockEnd) => PhysicalSide::Right,
        }
    }
}
