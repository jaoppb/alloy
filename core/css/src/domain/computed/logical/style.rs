//! [`LogicalStyle`] — an element's logical property set (CSS Logical Properties L1).

use super::direction::Direction;
use super::edges::LogicalEdges;
use super::insets::LogicalInsets;
use super::mapping::WritingContext;
use super::sizing::LogicalSizing;
use super::writing_mode::WritingMode;

/// The complete set of flow-relative computed properties for an element.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct LogicalStyle {
    context: WritingContext,
    sizing: LogicalSizing,
    margin: LogicalEdges,
    padding: LogicalEdges,
    border: LogicalEdges,
    insets: LogicalInsets,
}

impl LogicalStyle {
    /// Initial logical values.
    #[must_use]
    pub const fn initial() -> Self {
        Self {
            context: WritingContext::new(WritingMode::HorizontalTb, Direction::Ltr),
            sizing: LogicalSizing::initial(),
            margin: LogicalEdges::ZERO,
            padding: LogicalEdges::ZERO,
            border: LogicalEdges::ZERO,
            insets: LogicalInsets::AUTO,
        }
    }

    #[must_use]
    pub const fn context(self) -> WritingContext {
        self.context
    }

    #[must_use]
    pub const fn sizing(self) -> LogicalSizing {
        self.sizing
    }

    #[must_use]
    pub const fn margin(self) -> LogicalEdges {
        self.margin
    }

    #[must_use]
    pub const fn padding(self) -> LogicalEdges {
        self.padding
    }

    #[must_use]
    pub const fn border(self) -> LogicalEdges {
        self.border
    }

    #[must_use]
    pub const fn insets(self) -> LogicalInsets {
        self.insets
    }

    #[must_use]
    pub const fn with_context(self, context: WritingContext) -> Self {
        Self { context, ..self }
    }

    #[must_use]
    pub const fn with_sizing(self, sizing: LogicalSizing) -> Self {
        Self { sizing, ..self }
    }

    #[must_use]
    pub const fn with_margin(self, margin: LogicalEdges) -> Self {
        Self { margin, ..self }
    }

    #[must_use]
    pub const fn with_padding(self, padding: LogicalEdges) -> Self {
        Self { padding, ..self }
    }

    #[must_use]
    pub const fn with_border(self, border: LogicalEdges) -> Self {
        Self { border, ..self }
    }

    #[must_use]
    pub const fn with_insets(self, insets: LogicalInsets) -> Self {
        Self { insets, ..self }
    }
}
