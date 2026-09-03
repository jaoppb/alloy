//! Source-position value object used by diagnostic errors (PRD-002 invariant 4:
//! "structured Rust errors with line/column metadata").

use core::fmt;

/// A 1-based line number in a script source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Line(u32);

impl Line {
    /// Wrap a 1-based line number.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A 1-based column number in a script source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Column(u32);

impl Column {
    /// Wrap a 1-based column number.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Where in a script source a diagnostic points. An enum over what a backend can
/// actually report: some give `line:column`, some only the line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SourceLocation {
    /// Both coordinates known.
    LineColumn { line: Line, column: Column },
    /// Only the line is known.
    LineOnly { line: Line },
}

impl SourceLocation {
    /// A concrete `line:column`. Both are 1-based.
    #[must_use]
    pub const fn new(line: u32, column: u32) -> Self {
        Self::LineColumn {
            line: Line::new(line),
            column: Column::new(column),
        }
    }

    /// Only the line is known.
    #[must_use]
    pub const fn line_only(line: u32) -> Self {
        Self::LineOnly {
            line: Line::new(line),
        }
    }

    #[must_use]
    pub const fn line(&self) -> Line {
        match self {
            Self::LineColumn { line, .. } | Self::LineOnly { line } => *line,
        }
    }

    /// The column, when the backend reported one.
    #[must_use]
    pub const fn column(&self) -> Option<Column> {
        match self {
            Self::LineColumn { column, .. } => Some(*column),
            Self::LineOnly { .. } => None,
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LineColumn { line, column } => {
                write!(formatter, "line {}, column {}", line.get(), column.get())
            }
            Self::LineOnly { line } => write!(formatter, "line {}", line.get()),
        }
    }
}
