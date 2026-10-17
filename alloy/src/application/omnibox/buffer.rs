//! Text buffer and cursor position primitive for the browser omnibox.
//!
//! Enforces safe UTF-8 scalar navigation without raw bracket indexing,
//! honoring `string_slice = "deny"` and `indexing_slicing = "deny"`.

use core::fmt;

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
    text: String,
    cursor: CursorPosition,
}

impl OmniboxBuffer {
    /// Create an empty buffer with cursor at the start.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            text: String::new(),
            cursor: CursorPosition::ZERO,
        }
    }

    /// Create a buffer populated with initial text, placing the cursor at the end.
    #[must_use]
    pub fn from_text(initial_text: &str) -> Self {
        let text = initial_text.to_owned();
        let cursor = CursorPosition::new(text.len());
        Self { text, cursor }
    }

    /// The entire buffer text as a string slice.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The current cursor position.
    #[must_use]
    pub const fn cursor_position(&self) -> CursorPosition {
        self.cursor
    }

    /// Whether the buffer contains zero characters.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Total byte length of the buffer.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.text.len()
    }

    /// Total count of Unicode scalar values in the buffer.
    #[must_use]
    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    /// Number of Unicode scalar values preceding the cursor.
    #[must_use]
    pub fn cursor_char_index(&self) -> usize {
        self.text_before_cursor().chars().count()
    }

    /// Slice of text preceding the cursor, safe against UTF-8 boundaries.
    #[must_use]
    pub fn text_before_cursor(&self) -> &str {
        self.text.get(..self.cursor.byte_offset()).unwrap_or("")
    }

    /// Slice of text following the cursor, safe against UTF-8 boundaries.
    #[must_use]
    pub fn text_after_cursor(&self) -> &str {
        self.text.get(self.cursor.byte_offset()..).unwrap_or("")
    }

    /// Insert text at the current cursor position, advancing the cursor.
    ///
    /// Strips carriage returns and line feeds to ensure single-line invariant.
    pub fn insert_text(&mut self, text_to_insert: &str) {
        if text_to_insert.is_empty() {
            return;
        }
        let sanitized: String = text_to_insert
            .chars()
            .filter(|&character| character != '\r' && character != '\n')
            .collect();
        if sanitized.is_empty() {
            return;
        }
        let offset = self.cursor.byte_offset();
        self.text.insert_str(offset, &sanitized);
        self.cursor = CursorPosition::new(offset.saturating_add(sanitized.len()));
    }

    /// Delete the character immediately preceding the cursor (Backspace).
    pub fn backspace(&mut self) {
        if self.cursor.byte_offset() == 0 {
            return;
        }
        let previous_boundary = self
            .text
            .char_indices()
            .rev()
            .find(|&(offset, _)| offset < self.cursor.byte_offset());

        let Some((previous_offset, _)) = previous_boundary else {
            return;
        };

        self.text.remove(previous_offset);
        self.cursor = CursorPosition::new(previous_offset);
    }

    /// Delete the character immediately following the cursor (Forward Delete).
    pub fn delete(&mut self) {
        let offset = self.cursor.byte_offset();
        if offset >= self.text.len() {
            return;
        }
        self.text.remove(offset);
    }

    /// Move the cursor one character to the left.
    pub fn move_left(&mut self) {
        if self.cursor.byte_offset() == 0 {
            return;
        }
        let previous_boundary = self
            .text
            .char_indices()
            .rev()
            .find(|&(offset, _)| offset < self.cursor.byte_offset());

        if let Some((previous_offset, _)) = previous_boundary {
            self.cursor = CursorPosition::new(previous_offset);
        }
    }

    /// Move the cursor one character to the right.
    pub fn move_right(&mut self) {
        if self.cursor.byte_offset() >= self.text.len() {
            return;
        }
        let next_boundary = self
            .text
            .char_indices()
            .find(|&(offset, _)| offset > self.cursor.byte_offset());

        match next_boundary {
            Some((next_offset, _)) => self.cursor = CursorPosition::new(next_offset),
            None => self.cursor = CursorPosition::new(self.text.len()),
        }
    }

    /// Move the cursor to the beginning of the buffer.
    pub const fn move_start(&mut self) {
        self.cursor = CursorPosition::ZERO;
    }

    /// Move the cursor to the end of the buffer.
    pub const fn move_end(&mut self) {
        self.cursor = CursorPosition::new(self.text.len());
    }

    /// Clear all text and reset cursor to zero.
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = CursorPosition::ZERO;
    }

    /// Replace buffer content and position cursor at the end.
    pub fn set_text(&mut self, new_text: &str) {
        new_text.clone_into(&mut self.text);
        self.cursor = CursorPosition::new(self.text.len());
    }
}
