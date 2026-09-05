//! Source code position metadata for HTML tokenization and diagnostics.

use core::fmt;

/// Represents a source code position with line, column, and byte offset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceLocation {
    line: usize,
    column: usize,
    byte_offset: usize,
}

impl SourceLocation {
    /// Creates a source location at the start of input (line 1, column 1, offset 0).
    #[must_use]
    pub const fn initial() -> Self {
        Self {
            line: 1,
            column: 1,
            byte_offset: 0,
        }
    }

    /// Creates an explicit source location.
    #[must_use]
    pub const fn new(line: usize, column: usize, byte_offset: usize) -> Self {
        Self {
            line,
            column,
            byte_offset,
        }
    }

    /// 1-indexed line number.
    #[must_use]
    pub const fn line(&self) -> usize {
        self.line
    }

    /// 1-indexed column number.
    #[must_use]
    pub const fn column(&self) -> usize {
        self.column
    }

    /// 0-indexed byte offset in source string.
    #[must_use]
    pub const fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    /// Advances the location by one character.
    #[must_use]
    pub const fn advance(&self, character: char) -> Self {
        let byte_len = character.len_utf8();
        let new_offset = self.byte_offset.saturating_add(byte_len);
        if character == '\n' {
            return Self {
                line: self.line.saturating_add(1),
                column: 1,
                byte_offset: new_offset,
            };
        }
        Self {
            line: self.line,
            column: self.column.saturating_add(1),
            byte_offset: new_offset,
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}:offset {}",
            self.line, self.column, self.byte_offset
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_location_starts_at_first_position() {
        let location = SourceLocation::initial();
        assert_eq!(location.line(), 1);
        assert_eq!(location.column(), 1);
        assert_eq!(location.byte_offset(), 0);
    }

    #[test]
    fn advance_regular_character_increments_column_and_offset() {
        let location = SourceLocation::initial().advance('a');
        assert_eq!(location.line(), 1);
        assert_eq!(location.column(), 2);
        assert_eq!(location.byte_offset(), 1);
    }

    #[test]
    fn advance_newline_increments_line_and_resets_column() {
        let location = SourceLocation::initial().advance('\n');
        assert_eq!(location.line(), 2);
        assert_eq!(location.column(), 1);
        assert_eq!(location.byte_offset(), 1);
    }

    #[test]
    fn display_formats_location_correctly() {
        let location = SourceLocation::new(3, 14, 42);
        assert_eq!(location.to_string(), "3:14:offset 42");
    }
}
