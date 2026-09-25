//! Streaming character cursor with accurate source location tracking.

use crate::domain::location::SourceLocation;
use core::borrow::Borrow;
use std::borrow::Cow;

/// A cursor over a UTF-8 character sequence tracking source location.
#[derive(Clone, Debug)]
pub struct Cursor<'a> {
    source: Cow<'a, str>,
    byte_offset: usize,
    line: usize,
    column: usize,
    reconsumed: Option<char>,
}

impl<'a> Cursor<'a> {
    /// Create a cursor at the start of input.
    #[must_use]
    pub const fn new(source: &'a str) -> Self {
        Self {
            source: Cow::Borrowed(source),
            byte_offset: 0,
            line: 1,
            column: 1,
            reconsumed: None,
        }
    }

    /// Create a cursor from a Cow string with custom line and column.
    #[must_use]
    pub const fn from_cow(source: Cow<'a, str>, line: usize, column: usize) -> Self {
        Self {
            source,
            byte_offset: 0,
            line,
            column,
            reconsumed: None,
        }
    }

    /// Current source location.
    #[must_use]
    pub const fn location(&self) -> SourceLocation {
        SourceLocation::new(self.line, self.column, self.byte_offset)
    }

    /// Current byte offset.
    #[must_use]
    pub const fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    /// Peek the next character without advancing the cursor.
    #[must_use]
    pub fn peek(&self) -> Option<char> {
        if let Some(character) = self.reconsumed {
            return Some(character);
        }
        self.remaining().chars().next()
    }

    /// Advance and return the next character.
    pub fn next_char(&mut self) -> Option<char> {
        if let Some(character) = self.reconsumed.take() {
            return Some(character);
        }
        let character = self.remaining().chars().next()?;
        self.advance_by_char(character);
        Some(character)
    }

    /// Push back a character to be reconsumed on next read.
    pub const fn reconsume(&mut self, character: char) {
        self.reconsumed = Some(character);
    }

    /// Remaining source text slice.
    #[must_use]
    pub fn remaining(&self) -> &str {
        let text: &str = self.source.borrow();
        if self.byte_offset >= text.len() {
            return "";
        }
        text.get(self.byte_offset..).unwrap_or("")
    }

    const fn advance_by_char(&mut self, character: char) {
        let length = character.len_utf8();
        self.byte_offset = self.byte_offset.saturating_add(length);
        if character == '\n' {
            self.line = self.line.saturating_add(1);
            self.column = 1;
        } else {
            self.column = self.column.saturating_add(1);
        }
    }
}
