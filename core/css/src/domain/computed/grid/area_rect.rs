//! Bounding rectangle of a named grid area in 1-based line coordinates.

/// Bounding rectangle of a named grid area in 1-based line coordinates (CSS Grid L1 §7.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridAreaRect {
    row_start: u32,
    row_end: u32,
    column_start: u32,
    column_end: u32,
}

impl GridAreaRect {
    /// Constructs a grid area rectangle with 1-based line boundaries.
    #[must_use]
    pub const fn new(row_start: u32, column_start: u32, row_end: u32, column_end: u32) -> Self {
        Self {
            row_start,
            row_end,
            column_start,
            column_end,
        }
    }

    /// The 1-based start row line index.
    #[must_use]
    pub const fn row_start(self) -> u32 {
        self.row_start
    }

    /// The 1-based end row line index.
    #[must_use]
    pub const fn row_end(self) -> u32 {
        self.row_end
    }

    /// The 1-based start column line index.
    #[must_use]
    pub const fn column_start(self) -> u32 {
        self.column_start
    }

    /// The 1-based end column line index.
    #[must_use]
    pub const fn column_end(self) -> u32 {
        self.column_end
    }

    /// Total rows spanned by this area.
    #[must_use]
    pub const fn row_span(self) -> u32 {
        self.row_end.saturating_sub(self.row_start)
    }

    /// Total columns spanned by this area.
    #[must_use]
    pub const fn column_span(self) -> u32 {
        self.column_end.saturating_sub(self.column_start)
    }
}
