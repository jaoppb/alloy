//! What the omnibox is told and what it asks for.

use network::Url;
use window::InputText;

use super::EditKey;

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
///
/// The interim shape of `core/ui`'s `UiInput` (epic #110): the event loop's
/// key mapping produces these until the router and keymap land (#113).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OmniboxInput {
    /// User triggered the focus address bar shortcut (e.g. Ctrl+L or Alt+D).
    Focus,
    /// Text typed, pasted or committed by an input method.
    Insert(InputText),
    /// Editing, navigation, submit or cancel key (Backspace, Delete, Arrows,
    /// Home, End, Enter, Escape).
    Edit(EditKey),
}
