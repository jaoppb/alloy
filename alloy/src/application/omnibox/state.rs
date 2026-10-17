//! Omnibox interactive state machine aggregate.

use network::Url;

use super::buffer::{CursorPosition, OmniboxBuffer};

/// Interactive state machine for the browser address bar.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OmniboxState {
    is_focused: bool,
    is_all_selected: bool,
    buffer: OmniboxBuffer,
    active_url: Option<Url>,
}

impl OmniboxState {
    /// Create a new resting omnibox state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            is_focused: false,
            is_all_selected: false,
            buffer: OmniboxBuffer::new(),
            active_url: None,
        }
    }

    /// Create an omnibox state initialised with the active page's URL.
    #[must_use]
    pub fn with_url(url: Url) -> Self {
        let buffer = OmniboxBuffer::from_text(&url.to_text());
        Self {
            is_focused: false,
            is_all_selected: false,
            buffer,
            active_url: Some(url),
        }
    }

    /// Whether the omnibox input field currently holds user focus.
    #[must_use]
    pub const fn is_focused(&self) -> bool {
        self.is_focused
    }

    /// Whether the entire buffer text is currently selected.
    #[must_use]
    pub const fn is_all_selected(&self) -> bool {
        self.is_all_selected
    }

    /// The active canonical URL of the loaded page, if any.
    #[must_use]
    pub const fn active_url(&self) -> Option<&Url> {
        self.active_url.as_ref()
    }

    /// Read-only reference to the inner text buffer.
    #[must_use]
    pub const fn buffer(&self) -> &OmniboxBuffer {
        &self.buffer
    }

    /// The current text in the buffer.
    #[must_use]
    pub fn text(&self) -> &str {
        self.buffer.text()
    }

    /// The current cursor position.
    #[must_use]
    pub const fn cursor_position(&self) -> CursorPosition {
        self.buffer.cursor_position()
    }

    /// Grant focus to the omnibox.
    pub const fn focus(&mut self) {
        self.is_focused = true;
    }

    /// Remove focus from the omnibox and clear selection.
    pub const fn blur(&mut self) {
        self.is_focused = false;
        self.is_all_selected = false;
    }

    /// Select all text in the buffer.
    pub const fn select_all(&mut self) {
        self.is_all_selected = true;
    }

    /// Clear text selection.
    pub const fn clear_selection(&mut self) {
        self.is_all_selected = false;
    }

    /// Insert text into the omnibox, replacing any active selection.
    pub fn insert_text(&mut self, text: &str) {
        if self.is_all_selected {
            self.buffer.clear();
            self.is_all_selected = false;
        }
        self.buffer.insert_text(text);
    }

    /// Backspace deletion, removing all text if selected or one char preceding caret.
    pub fn backspace(&mut self) {
        if self.is_all_selected {
            self.buffer.clear();
            self.is_all_selected = false;
            return;
        }
        self.buffer.backspace();
    }

    /// Forward delete, removing all text if selected or one char following caret.
    pub fn delete(&mut self) {
        if self.is_all_selected {
            self.buffer.clear();
            self.is_all_selected = false;
            return;
        }
        self.buffer.delete();
    }

    /// Move cursor left, clearing selection and placing cursor at start if selected.
    pub fn move_left(&mut self) {
        if self.is_all_selected {
            self.is_all_selected = false;
            self.buffer.move_start();
            return;
        }
        self.buffer.move_left();
    }

    /// Move cursor right, clearing selection and placing cursor at end if selected.
    pub fn move_right(&mut self) {
        if self.is_all_selected {
            self.is_all_selected = false;
            self.buffer.move_end();
            return;
        }
        self.buffer.move_right();
    }

    /// Move cursor to the start of the buffer.
    pub const fn move_start(&mut self) {
        self.is_all_selected = false;
        self.buffer.move_start();
    }

    /// Move cursor to the end of the buffer.
    pub const fn move_end(&mut self) {
        self.is_all_selected = false;
        self.buffer.move_end();
    }

    /// Reset the buffer with the canonical URL and clear focus and selection state.
    pub fn reset_to_url(&mut self, url: &Url) {
        self.active_url = Some(url.clone());
        self.buffer.set_text(&url.to_text());
        self.is_focused = false;
        self.is_all_selected = false;
    }

    /// Revert any in-progress edits to the active page URL and clear focus.
    pub fn cancel_editing(&mut self) {
        if let Some(active_url) = self.active_url.as_ref() {
            self.buffer.set_text(&active_url.to_text());
        }
        self.blur();
    }
}
