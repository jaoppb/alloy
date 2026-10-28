//! [`KeyboardInput`] — the window events the chrome's router consumes.

use window::{InputText, KeyEvent, WindowEvent};

/// A key press or an input-method commit, as the window reported it. The loop
/// does not interpret either: the chrome's router decides what each means
/// (`ADR-0027`). The type is the filter, so a `Resized` can never reach it.
pub enum KeyboardInput {
    Key(KeyEvent),
    Committed(InputText),
}

impl KeyboardInput {
    /// The keyboard input `event` carries, or `None` for any other event.
    pub fn from_event(event: &WindowEvent) -> Option<Self> {
        match event {
            WindowEvent::Key(key) => Some(Self::Key(key.clone())),
            WindowEvent::TextCommitted(text) => Some(Self::Committed(text.clone())),
            _ => None,
        }
    }
}
