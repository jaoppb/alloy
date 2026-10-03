//! [`LogicalSizing`] — sizing along the inline and block axes (CSS Logical Properties L1 §5).

use crate::domain::computed::sizing::Sizing;

use super::axis::{LogicalAxis, PhysicalAxis};
use super::mapping::WritingContext;

/// Sizing dimensions along the block and inline axes.
#[allow(clippy::struct_field_names)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct LogicalSizing {
    inline_size: Sizing,
    block_size: Sizing,
    min_inline_size: Sizing,
    min_block_size: Sizing,
    max_inline_size: Sizing,
    max_block_size: Sizing,
}

impl LogicalSizing {
    /// Initial logical sizing values (`auto` for sizes and constraints).
    #[must_use]
    pub const fn initial() -> Self {
        Self {
            inline_size: Sizing::Auto,
            block_size: Sizing::Auto,
            min_inline_size: Sizing::Auto,
            min_block_size: Sizing::Auto,
            max_inline_size: Sizing::Auto,
            max_block_size: Sizing::Auto,
        }
    }

    #[must_use]
    pub const fn inline_size(self) -> Sizing {
        self.inline_size
    }

    #[must_use]
    pub const fn block_size(self) -> Sizing {
        self.block_size
    }

    #[must_use]
    pub const fn min_inline_size(self) -> Sizing {
        self.min_inline_size
    }

    #[must_use]
    pub const fn min_block_size(self) -> Sizing {
        self.min_block_size
    }

    #[must_use]
    pub const fn max_inline_size(self) -> Sizing {
        self.max_inline_size
    }

    #[must_use]
    pub const fn max_block_size(self) -> Sizing {
        self.max_block_size
    }

    #[must_use]
    pub const fn with_inline_size(self, size: Sizing) -> Self {
        Self {
            inline_size: size,
            ..self
        }
    }

    #[must_use]
    pub const fn with_block_size(self, size: Sizing) -> Self {
        Self {
            block_size: size,
            ..self
        }
    }

    #[must_use]
    pub const fn with_min_inline_size(self, size: Sizing) -> Self {
        Self {
            min_inline_size: size,
            ..self
        }
    }

    #[must_use]
    pub const fn with_min_block_size(self, size: Sizing) -> Self {
        Self {
            min_block_size: size,
            ..self
        }
    }

    #[must_use]
    pub const fn with_max_inline_size(self, size: Sizing) -> Self {
        Self {
            max_inline_size: size,
            ..self
        }
    }

    #[must_use]
    pub const fn with_max_block_size(self, size: Sizing) -> Self {
        Self {
            max_block_size: size,
            ..self
        }
    }

    /// Resolves which physical dimension corresponds to the inline axis.
    #[must_use]
    pub const fn inline_physical_axis(context: WritingContext) -> PhysicalAxis {
        context.map_axis(LogicalAxis::Inline)
    }

    /// Resolves which physical dimension corresponds to the block axis.
    #[must_use]
    pub const fn block_physical_axis(context: WritingContext) -> PhysicalAxis {
        context.map_axis(LogicalAxis::Block)
    }
}
