//! Gap buffer text storage primitive for the omnibox.
//!
//! Provides amortized O(1) text insertions and deletions at the cursor position
//! by maintaining an unfilled gap between text preceding and succeeding the cursor.
//! Guarantees safe UTF-8 scalar boundaries without raw slice indexing.

use std::borrow::Cow;

const DEFAULT_GAP_CAPACITY: usize = 64;

/// A contiguous byte buffer with a movable editing gap on UTF-8 scalar boundaries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GapBuffer {
    data: Vec<u8>,
    gap_start: usize,
    gap_end: usize,
}

impl Default for GapBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl GapBuffer {
    /// Create an empty gap buffer with pre-allocated gap capacity.
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: vec![0; DEFAULT_GAP_CAPACITY],
            gap_start: 0,
            gap_end: DEFAULT_GAP_CAPACITY,
        }
    }

    /// Create a gap buffer initialized with text, placing the gap and cursor at the end.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        let bytes = text.as_bytes();
        let text_len = bytes.len();
        let total_capacity = text_len.saturating_add(DEFAULT_GAP_CAPACITY);
        let mut data = vec![0; total_capacity];
        if let Some(target) = data.get_mut(..text_len) {
            target.copy_from_slice(bytes);
        }
        Self {
            data,
            gap_start: text_len,
            gap_end: total_capacity,
        }
    }

    /// The cursor position as a byte offset within the logical text.
    #[must_use]
    pub const fn cursor_byte_offset(&self) -> usize {
        self.gap_start
    }

    /// Total byte length of the logical text, excluding the gap.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.gap_start
            .saturating_add(self.data.len().saturating_sub(self.gap_end))
    }

    /// Whether the logical text contains zero bytes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Count of Unicode scalar values in the buffer.
    #[must_use]
    pub fn char_count(&self) -> usize {
        self.text_before_cursor()
            .chars()
            .count()
            .saturating_add(self.text_after_cursor().chars().count())
    }

    /// Count of Unicode scalar values preceding the cursor.
    #[must_use]
    pub fn cursor_char_index(&self) -> usize {
        self.text_before_cursor().chars().count()
    }

    /// Contiguous string slice preceding the cursor.
    #[must_use]
    pub fn text_before_cursor(&self) -> &str {
        let Some(slice) = self.data.get(..self.gap_start) else {
            return "";
        };
        core::str::from_utf8(slice).unwrap_or("")
    }

    /// Contiguous string slice following the cursor.
    #[must_use]
    pub fn text_after_cursor(&self) -> &str {
        let Some(slice) = self.data.get(self.gap_end..) else {
            return "";
        };
        core::str::from_utf8(slice).unwrap_or("")
    }

    /// Full buffer text, borrowed if cursor is at an edge or concatenated if split.
    #[must_use]
    pub fn text(&self) -> Cow<'_, str> {
        if self.gap_end >= self.data.len() {
            return Cow::Borrowed(self.text_before_cursor());
        }
        if self.gap_start == 0 {
            return Cow::Borrowed(self.text_after_cursor());
        }
        let mut combined = String::with_capacity(self.len());
        combined.push_str(self.text_before_cursor());
        combined.push_str(self.text_after_cursor());
        Cow::Owned(combined)
    }

    /// Insert text at the current cursor position, expanding the gap if necessary.
    pub fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let bytes = text.as_bytes();
        let needed = bytes.len();
        self.ensure_gap_capacity(needed);

        let target_end = self.gap_start.saturating_add(needed);
        let Some(target) = self.data.get_mut(self.gap_start..target_end) else {
            return;
        };
        target.copy_from_slice(bytes);
        self.gap_start = target_end;
    }

    /// Delete one Unicode scalar preceding the cursor (Backspace).
    pub fn backspace(&mut self) {
        if self.gap_start == 0 {
            return;
        }
        let Some((prev_offset, _)) = self.text_before_cursor().char_indices().next_back() else {
            return;
        };
        self.gap_start = prev_offset;
    }

    /// Delete one Unicode scalar following the cursor (Forward Delete).
    pub fn delete(&mut self) {
        if self.gap_end >= self.data.len() {
            return;
        }
        let Some(first_char) = self.text_after_cursor().chars().next() else {
            return;
        };
        self.gap_end = self.gap_end.saturating_add(first_char.len_utf8());
    }

    /// Move the cursor and gap one Unicode scalar to the left.
    pub fn move_left(&mut self) {
        if self.gap_start == 0 {
            return;
        }
        let Some((prev_offset, _)) = self.text_before_cursor().char_indices().next_back() else {
            return;
        };
        let char_len = self.gap_start.saturating_sub(prev_offset);
        let new_gap_end = self.gap_end.saturating_sub(char_len);

        self.data
            .copy_within(prev_offset..self.gap_start, new_gap_end);
        self.gap_start = prev_offset;
        self.gap_end = new_gap_end;
    }

    /// Move the cursor and gap one Unicode scalar to the right.
    pub fn move_right(&mut self) {
        if self.gap_end >= self.data.len() {
            return;
        }
        let Some(first_char) = self.text_after_cursor().chars().next() else {
            return;
        };
        let char_len = first_char.len_utf8();
        let next_char_end = self.gap_end.saturating_add(char_len);

        self.data
            .copy_within(self.gap_end..next_char_end, self.gap_start);
        self.gap_start = self.gap_start.saturating_add(char_len);
        self.gap_end = next_char_end;
    }

    /// Move cursor and gap to the beginning of the buffer.
    pub fn move_start(&mut self) {
        if self.gap_start == 0 {
            return;
        }
        let prefix_len = self.gap_start;
        let new_gap_end = self.gap_end.saturating_sub(prefix_len);
        self.data.copy_within(0..prefix_len, new_gap_end);
        self.gap_start = 0;
        self.gap_end = new_gap_end;
    }

    /// Move cursor and gap to the end of the buffer.
    pub fn move_end(&mut self) {
        if self.gap_end >= self.data.len() {
            return;
        }
        let suffix_len = self.data.len().saturating_sub(self.gap_end);
        let old_len = self.data.len();
        self.data.copy_within(self.gap_end..old_len, self.gap_start);
        self.gap_start = self.gap_start.saturating_add(suffix_len);
        self.gap_end = old_len;
    }

    /// Clear all text and reset the gap across the entire allocated buffer.
    pub const fn clear(&mut self) {
        self.gap_start = 0;
        self.gap_end = self.data.len();
    }

    /// Replace buffer content with new text, placing cursor at the end.
    pub fn set_text(&mut self, text: &str) {
        *self = Self::from_text(text);
    }

    fn ensure_gap_capacity(&mut self, needed: usize) {
        let current_gap = self.gap_end.saturating_sub(self.gap_start);
        if current_gap >= needed {
            return;
        }
        let additional = needed.saturating_add(DEFAULT_GAP_CAPACITY);
        let old_len = self.data.len();
        let new_len = old_len.saturating_add(additional);
        self.data.resize(new_len, 0);

        let suffix_len = old_len.saturating_sub(self.gap_end);
        let new_gap_end = self.gap_end.saturating_add(additional);

        if suffix_len > 0 {
            self.data.copy_within(self.gap_end..old_len, new_gap_end);
        }
        self.gap_end = new_gap_end;
    }
}
