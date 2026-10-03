//! Grid gutters / gap properties (CSS Box Alignment L3 §8).

use core::fmt;

use crate::domain::length::Length;

/// Grid gutter dimensions for rows and columns.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GridGap {
    row: Length,
    column: Length,
}

impl GridGap {
    /// Zero gutters.
    pub const ZERO: Self = Self::uniform(Length::ZERO);

    /// Constructs identical row and column gaps.
    #[must_use]
    pub const fn uniform(gap: Length) -> Self {
        Self {
            row: gap,
            column: gap,
        }
    }

    /// Constructs distinct row and column gaps.
    #[must_use]
    pub const fn new(row: Length, column: Length) -> Self {
        Self { row, column }
    }

    /// The row gap.
    #[must_use]
    pub const fn row(self) -> Length {
        self.row
    }

    /// The column gap.
    #[must_use]
    pub const fn column(self) -> Length {
        self.column
    }

    /// Builder with updated row gap.
    #[must_use]
    pub const fn with_row(self, row: Length) -> Self {
        Self { row, ..self }
    }

    /// Builder with updated column gap.
    #[must_use]
    pub const fn with_column(self, column: Length) -> Self {
        Self { column, ..self }
    }
}

impl fmt::Display for GridGap {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.row == self.column {
            return write!(formatter, "{}", self.row);
        }
        write!(formatter, "{} {}", self.row, self.column)
    }
}
