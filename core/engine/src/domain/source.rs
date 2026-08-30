//! Source-position value object used by diagnostic errors (PRD-002 invariant 4:
//! "structured Rust errors with line/column metadata").

use core::fmt;

/// A 1-based position inside a script source. Column 0 means "column unknown but
/// the line is known" — some backends only surface line granularity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    line: u32,
    column: u32,
}

impl SourceLocation {
    /// A concrete `line:column`. Both are 1-based.
    #[must_use]
    pub const fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }

    /// Only the line is known.
    #[must_use]
    pub const fn line_only(line: u32) -> Self {
        Self { line, column: 0 }
    }

    #[must_use]
    pub const fn line(&self) -> u32 {
        self.line
    }

    #[must_use]
    pub const fn column(&self) -> u32 {
        self.column
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.column {
            0 => write!(formatter, "line {}", self.line),
            column => write!(formatter, "line {}, column {column}", self.line),
        }
    }
}
