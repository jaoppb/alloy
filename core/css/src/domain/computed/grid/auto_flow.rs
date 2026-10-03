//! Grid auto-placement flow (CSS Grid L1 §7.7).

use core::fmt;

/// Controls how auto-placed grid items get inserted into the grid (CSS Grid L1 §7.7).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GridAutoFlow {
    /// Placed by filling each row in turn.
    #[default]
    Row,
    /// Placed by filling each column in turn.
    Column,
    /// "Dense" packing algorithm: attempt to fill earlier holes.
    RowDense,
    /// "Dense" packing algorithm for columns.
    ColumnDense,
}

impl GridAutoFlow {
    /// Whether the major axis is rows.
    #[must_use]
    pub const fn is_row(self) -> bool {
        matches!(self, Self::Row | Self::RowDense)
    }

    /// Whether the major axis is columns.
    #[must_use]
    pub const fn is_column(self) -> bool {
        matches!(self, Self::Column | Self::ColumnDense)
    }

    /// Whether dense packing is requested.
    #[must_use]
    pub const fn is_dense(self) -> bool {
        matches!(self, Self::RowDense | Self::ColumnDense)
    }

    /// The CSS keyword representation.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Row => "row",
            Self::Column => "column",
            Self::RowDense => "row dense",
            Self::ColumnDense => "column dense",
        }
    }
}

impl fmt::Display for GridAutoFlow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}
