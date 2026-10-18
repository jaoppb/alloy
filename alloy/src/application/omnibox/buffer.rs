//! Text buffer and cursor position primitive for the browser omnibox.
//!
//! Enforces safe UTF-8 scalar navigation without raw bracket indexing,
//! backed by a [`GapBuffer`] for amortized O(1) interactive editing.

use core::fmt;
use std::borrow::Cow;

use super::gap_buffer::GapBuffer;

/// A strongly-typed byte offset representing an insertion point in an [`OmniboxBuffer`].
///
/// Guaranteed to lie on a valid UTF-8 scalar boundary within the buffer's text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CursorPosition(usize);

impl CursorPosition {
    /// Beginning of buffer.
    pub const ZERO: Self = Self(0);

    /// Create a cursor position at a verified byte offset.
    #[must_use]
    pub const fn new(byte_offset: usize) -> Self {
        Self(byte_offset)
    }

    /// The byte offset into the UTF-8 text buffer.
    #[must_use]
    pub const fn byte_offset(self) -> usize {
        self.0
    }
}

impl fmt::Display for CursorPosition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "cursor@{}", self.0)
    }
}

/// An interactive single-line text buffer with a safe UTF-8 cursor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OmniboxBuffer {
    gap_buffer: GapBuffer,
}

impl OmniboxBuffer {
    /// Create an empty buffer with cursor at the start.
    #[must_use]
    pub fn new() -> Self {
        Self {
            gap_buffer: GapBuffer::new(),
        }
    }

    /// Create a buffer populated with initial text, placing the cursor at the end.
    ///
    /// Strips carriage returns and line feeds to enforce single-line invariant.
    #[must_use]
    pub fn from_text(initial_text: &str) -> Self {
        let sanitized = sanitize_single_line(initial_text);
        Self {
            gap_buffer: GapBuffer::from_text(&sanitized),
        }
    }

    /// The entire buffer text, borrowed when contiguous or combined when split across gap.
    #[must_use]
    pub fn text(&self) -> Cow<'_, str> {
        self.gap_buffer.text()
    }

    /// The current cursor position.
    #[must_use]
    pub const fn cursor_position(&self) -> CursorPosition {
        CursorPosition::new(self.gap_buffer.cursor_byte_offset())
    }

    /// Whether the buffer contains zero characters.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.gap_buffer.is_empty()
    }

    /// Total byte length of the buffer.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.gap_buffer.len()
    }

    /// Total count of Unicode scalar values in the buffer.
    #[must_use]
    pub fn char_count(&self) -> usize {
        self.gap_buffer.char_count()
    }

    /// Number of Unicode scalar values preceding the cursor.
    #[must_use]
    pub fn cursor_char_index(&self) -> usize {
        self.gap_buffer.cursor_char_index()
    }

    /// Slice of text preceding the cursor, safe against UTF-8 boundaries.
    #[must_use]
    pub fn text_before_cursor(&self) -> &str {
        self.gap_buffer.text_before_cursor()
    }

    /// Slice of text following the cursor, safe against UTF-8 boundaries.
    #[must_use]
    pub fn text_after_cursor(&self) -> &str {
        self.gap_buffer.text_after_cursor()
    }

    /// Insert text at the current cursor position, advancing the cursor.
    ///
    /// Strips carriage returns and line feeds to ensure single-line invariant.
    pub fn insert_text(&mut self, text_to_insert: &str) {
        let sanitized = sanitize_single_line(text_to_insert);
        self.gap_buffer.insert_text(&sanitized);
    }

    /// Delete the character immediately preceding the cursor (Backspace).
    pub fn backspace(&mut self) {
        self.gap_buffer.backspace();
    }

    /// Delete the character immediately following the cursor (Forward Delete).
    pub fn delete(&mut self) {
        self.gap_buffer.delete();
    }

    /// Move the cursor one character to the left.
    pub fn move_left(&mut self) {
        self.gap_buffer.move_left();
    }

    /// Move the cursor one character to the right.
    pub fn move_right(&mut self) {
        self.gap_buffer.move_right();
    }

    /// Move the cursor to the beginning of the buffer.
    pub fn move_start(&mut self) {
        self.gap_buffer.move_start();
    }

    /// Move the cursor to the end of the buffer.
    pub fn move_end(&mut self) {
        self.gap_buffer.move_end();
    }

    /// Clear all text and reset cursor to zero.
    pub const fn clear(&mut self) {
        self.gap_buffer.clear();
    }

    /// Replace buffer content and position cursor at the end.
    ///
    /// Strips carriage returns and line feeds to enforce single-line invariant.
    pub fn set_text(&mut self, new_text: &str) {
        let sanitized = sanitize_single_line(new_text);
        self.gap_buffer.set_text(&sanitized);
    }
}

fn sanitize_single_line(input: &str) -> String {
    input
        .chars()
        .filter(|&character| character != '\r' && character != '\n')
        .collect()
}
