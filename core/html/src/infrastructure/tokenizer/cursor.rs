//! Streaming character cursor with accurate source location tracking.

use crate::domain::diagnostic::{ParseDiagnostic, ParseErrorCode};
use crate::domain::location::SourceLocation;
use core::borrow::Borrow;
use std::borrow::Cow;

/// A cursor over a UTF-8 character sequence tracking source location.
///
/// Every tokenizer handler already receives the cursor, so it also carries the diagnostics the
/// handlers report (ADR-0023); the tokenizer drains them ahead of the token they precede.
#[derive(Clone, Debug)]
pub struct Cursor<'a> {
    source: Cow<'a, str>,
    byte_offset: usize,
    line: usize,
    column: usize,
    reconsumed: Option<char>,
    last_start: SourceLocation,
    diagnostics: Vec<ParseDiagnostic>,
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
            last_start: SourceLocation::initial(),
            diagnostics: Vec::new(),
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
            last_start: SourceLocation::new(line, column, 0),
            diagnostics: Vec::new(),
        }
    }

    /// Current source location.
    #[must_use]
    pub const fn location(&self) -> SourceLocation {
        SourceLocation::new(self.line, self.column, self.byte_offset)
    }

    /// Where the most recently returned character started (also for a reconsumed one).
    #[must_use]
    pub const fn last_location(&self) -> SourceLocation {
        self.last_start
    }

    /// Queues a recoverable parse error at `location`.
    pub fn report(&mut self, code: ParseErrorCode, location: SourceLocation) {
        self.diagnostics.push(ParseDiagnostic::new(code, location));
    }

    /// Hands over the diagnostics queued since the last call, in order.
    pub fn take_diagnostics(&mut self) -> Vec<ParseDiagnostic> {
        core::mem::take(&mut self.diagnostics)
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
        self.last_start = self.location();
        self.report_input_stream_error(character);
        self.advance_by_char(character);
        Some(character)
    }

    /// WHATWG §13.2.5 input-stream preprocessing: NUL and non-whitespace controls are parse errors.
    fn report_input_stream_error(&mut self, character: char) {
        if character == '\0' {
            self.report(ParseErrorCode::UnexpectedNullCharacter, self.last_start);
            return;
        }
        let is_whitespace_control = matches!(character, '\t' | '\n' | '\u{000C}' | '\r');
        if character.is_control() && !is_whitespace_control {
            self.report(
                ParseErrorCode::ControlCharacterInInputStream,
                self.last_start,
            );
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_location_is_where_the_returned_character_started() {
        let mut cursor = Cursor::new("a\nb");
        cursor.next_char();
        cursor.next_char();
        cursor.next_char();
        assert_eq!(cursor.last_location(), SourceLocation::new(2, 1, 2));
    }

    #[test]
    fn a_reconsumed_character_keeps_its_original_start() {
        let mut cursor = Cursor::new("ab");
        cursor.next_char();
        let second = cursor.next_char().unwrap();
        cursor.reconsume(second);
        assert_eq!(cursor.next_char(), Some('b'));
        assert_eq!(cursor.last_location(), SourceLocation::new(1, 2, 1));
    }

    #[test]
    fn nul_and_control_characters_are_reported_once_per_fresh_character() {
        let mut cursor = Cursor::new("\0\u{1}\t");
        let first = cursor.next_char().unwrap();
        cursor.reconsume(first);
        cursor.next_char();
        cursor.next_char();
        cursor.next_char();
        let codes: Vec<_> = cursor
            .take_diagnostics()
            .iter()
            .map(ParseDiagnostic::code)
            .collect();
        assert_eq!(
            codes,
            [
                ParseErrorCode::UnexpectedNullCharacter,
                ParseErrorCode::ControlCharacterInInputStream
            ]
        );
        assert!(cursor.take_diagnostics().is_empty());
    }
}
