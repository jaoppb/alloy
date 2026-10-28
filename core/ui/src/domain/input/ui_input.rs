//! [`UiInput`] — key input after the router has decided what it means.

use window::InputText;

use super::BrowserCommand;
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
