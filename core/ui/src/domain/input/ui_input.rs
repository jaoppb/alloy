//! [`UiInput`] — key input after the router has decided what it means.

use window::InputText;

use super::{BrowserCommand, FocusTarget};
use crate::domain::omnibox::EditKey;

/// Resolved input: a controller never sees a raw key, a modifier or a chord.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiInput {
    /// A keymap binding fired.
    Command(BrowserCommand),
    /// A named editing key (caret movement, delete, submit, cancel).
    Edit(EditKey),
    /// Text typed, pasted or committed by an input method.
    Insert(InputText),
}

impl UiInput {
    /// Whether `target` consumes this input. Commands are the browser's, not
    /// a target's; typing and editing belong to the focused target alone.
    #[must_use]
    pub const fn reaches(&self, target: FocusTarget) -> bool {
        match self {
            Self::Command(_) => true,
            Self::Edit(_) | Self::Insert(_) => matches!(target, FocusTarget::Omnibox),
        }
    }
}
