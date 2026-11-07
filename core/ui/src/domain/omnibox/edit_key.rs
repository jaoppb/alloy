//! [`EditKey`] — the keys the omnibox edits, submits or cancels with.

use window::LogicalKey;

/// A key the omnibox acts on by itself: caret movement, deletion, submit and
/// cancel.
///
/// A closed enum so the set lives in one place: the input router converts a
/// [`LogicalKey`] once with [`Self::from_logical`] into a `UiInput::Edit`, and
/// the controller's `match` over it is exhaustive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKey {
    Enter,
    Escape,
    Backspace,
    Delete,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
}

impl EditKey {
    /// The edit key `key` stands for, or `None` for any other key.
    #[must_use]
    pub const fn from_logical(key: &LogicalKey) -> Option<Self> {
        match key {
            LogicalKey::Enter => Some(Self::Enter),
            LogicalKey::Escape => Some(Self::Escape),
            LogicalKey::Backspace => Some(Self::Backspace),
            LogicalKey::Delete => Some(Self::Delete),
            LogicalKey::ArrowLeft => Some(Self::ArrowLeft),
            LogicalKey::ArrowRight => Some(Self::ArrowRight),
            LogicalKey::Home => Some(Self::Home),
            LogicalKey::End => Some(Self::End),
            _ => None,
        }
    }
}
