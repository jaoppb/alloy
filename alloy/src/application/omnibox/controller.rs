//! Omnibox controller translating inputs into state transitions and high-level effects.

use std::borrow::Cow;

use network::{OmniboxClassifier, Url};
use window::LogicalKey;

use super::state::OmniboxState;

/// High-level effect produced by omnibox interactions that the session executes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OmniboxEffect {
    /// No side effects or visual modifications occurred.
    None,
    /// Omnibox state changed, requesting a UI redraw.
    Redraw,
    /// User submitted an address or search query to navigate to.
    Navigate(Url),
}

/// An input event targeted at the omnibox.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OmniboxInput {
    /// User triggered the focus address bar shortcut (e.g. Ctrl+L or Alt+D).
    Focus,
    /// Text typed or pasted into the address bar.
    TextInput(String),
    /// Control key pressed (e.g. Backspace, Delete, Arrows, Home, End).
    ControlKey(LogicalKey),
    /// Enter key pressed to submit input.
    Enter,
    /// Escape key pressed to cancel editing.
    Escape,
}

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
            OmniboxInput::TextInput(text) => self.handle_text_input(text),
            OmniboxInput::ControlKey(key) => self.handle_control_key(*key),
            OmniboxInput::Enter => self.handle_enter(),
            OmniboxInput::Escape => self.handle_escape(),
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

    /// Routes logical control key navigation/editing to the omnibox when active.
    pub fn handle_control_key(&mut self, key: LogicalKey) -> OmniboxEffect {
        if !self.state.is_focused() {
            return OmniboxEffect::None;
        }
        match key {
            LogicalKey::Enter => self.handle_enter(),
            LogicalKey::Escape => self.handle_escape(),
            LogicalKey::Backspace => self.execute_backspace(),
            LogicalKey::Delete => self.execute_delete(),
            LogicalKey::ArrowLeft => self.execute_move_left(),
            LogicalKey::ArrowRight => self.execute_move_right(),
            LogicalKey::Home => self.execute_move_start(),
            LogicalKey::End => self.execute_move_end(),
            _ => OmniboxEffect::None,
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
