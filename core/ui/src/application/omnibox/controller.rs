//! Omnibox controller translating inputs into state transitions and high-level effects.

use std::borrow::Cow;

use network::Url;

use crate::domain::omnibox::{
    EditKey, OmniboxClassifier, OmniboxEffect, OmniboxInput, OmniboxState,
};

/// Coordinates omnibox state mutations and translates inputs into high-level effects.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OmniboxController {
    state: OmniboxState,
    classifier: OmniboxClassifier,
}

impl OmniboxController {
    /// Create a new resting omnibox controller.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: OmniboxState::new(),
            classifier: OmniboxClassifier::default(),
        }
    }

    /// Create an omnibox controller initialised with the active page's URL.
    #[must_use]
    pub fn with_url(url: Url) -> Self {
        Self {
            state: OmniboxState::with_url(url),
            classifier: OmniboxClassifier::default(),
        }
    }

    /// Read-only reference to the inner omnibox state.
    #[must_use]
    pub const fn state(&self) -> &OmniboxState {
        &self.state
    }

    /// Whether the omnibox currently holds user focus.
    #[must_use]
    pub const fn is_focused(&self) -> bool {
        self.state.is_focused()
    }

    /// Whether the entire buffer text is currently selected.
    #[must_use]
    pub const fn is_all_selected(&self) -> bool {
        self.state.is_all_selected()
    }

    /// The current text in the buffer.
    #[must_use]
    pub fn text(&self) -> Cow<'_, str> {
        self.state.text()
    }

    /// Resets the omnibox buffer with a new canonical navigation URL.
    pub fn reset_to_url(&mut self, url: &Url) {
        self.state.reset_to_url(url);
    }

    /// Process an omnibox input and translate it into a high-level effect.
    pub fn handle_input(&mut self, input: &OmniboxInput) -> OmniboxEffect {
        match input {
            OmniboxInput::Focus => self.focus(),
            OmniboxInput::Insert(text) => self.handle_text_input(text.as_str()),
            OmniboxInput::Edit(key) => self.handle_edit_key(*key),
        }
    }

    /// Grants focus to the omnibox and selects all existing text.
    pub const fn focus(&mut self) -> OmniboxEffect {
        self.state.focus();
        self.state.select_all();
        OmniboxEffect::Redraw
    }

    /// Inserts typed text into the omnibox when focused.
    pub fn handle_text_input(&mut self, text: &str) -> OmniboxEffect {
        if !self.state.is_focused() {
            return OmniboxEffect::None;
        }
        self.state.insert_text(text);
        OmniboxEffect::Redraw
    }

    /// Cancels editing in the omnibox, reverts to active URL, and unfocuses.
    pub fn handle_escape(&mut self) -> OmniboxEffect {
        if !self.state.is_focused() {
            return OmniboxEffect::None;
        }
        self.state.cancel_editing();
        OmniboxEffect::Redraw
    }

    /// Submits the current omnibox input, classifying it into a URL or search query.
    pub fn handle_enter(&mut self) -> OmniboxEffect {
        if !self.state.is_focused() {
            return OmniboxEffect::None;
        }
        self.state.blur();
        let text = self.state.text();
        let Ok(destination) = self.classifier.classify(&text) else {
            return OmniboxEffect::Redraw;
        };
        OmniboxEffect::Navigate(destination.into_url())
    }

    /// Routes logical editing/navigation keys to the omnibox when active.
    pub fn handle_edit_key(&mut self, key: EditKey) -> OmniboxEffect {
        if !self.state.is_focused() {
            return OmniboxEffect::None;
        }
        match key {
            EditKey::Enter => self.handle_enter(),
            EditKey::Escape => self.handle_escape(),
            EditKey::Backspace => self.execute_backspace(),
            EditKey::Delete => self.execute_delete(),
            EditKey::ArrowLeft => self.execute_move_left(),
            EditKey::ArrowRight => self.execute_move_right(),
            EditKey::Home => self.execute_move_start(),
            EditKey::End => self.execute_move_end(),
        }
    }

    fn execute_backspace(&mut self) -> OmniboxEffect {
        self.state.backspace();
        OmniboxEffect::Redraw
    }

    fn execute_delete(&mut self) -> OmniboxEffect {
        self.state.delete();
        OmniboxEffect::Redraw
    }

    fn execute_move_left(&mut self) -> OmniboxEffect {
        self.state.move_left();
        OmniboxEffect::Redraw
    }

    fn execute_move_right(&mut self) -> OmniboxEffect {
        self.state.move_right();
        OmniboxEffect::Redraw
    }

    fn execute_move_start(&mut self) -> OmniboxEffect {
        self.state.move_start();
        OmniboxEffect::Redraw
    }

    fn execute_move_end(&mut self) -> OmniboxEffect {
        self.state.move_end();
        OmniboxEffect::Redraw
    }
}
