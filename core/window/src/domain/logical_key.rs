//! [`LogicalKey`] — what a key press *means* under the active layout.

use core::fmt;

use crate::domain::text::InputText;

/// The meaning of a key press under the active layout, as opposed to
/// [`KeyCode`](crate::domain::key::KeyCode), which is its position.
///
/// Holds the keys a keymap or an editing field routes on by name; every other
/// named key a backend can report is [`Self::Other`] — a consumer that needs
/// it falls back to the [`KeyEvent::physical`] position, and a new variant is
/// additive (`#[non_exhaustive]`).
///
/// [`KeyEvent::physical`]: crate::domain::key_event::KeyEvent::physical
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LogicalKey {
    /// A key that produces a character under the active layout, with every
    /// modifier except Ctrl applied (`winit` semantics): Shift+`a` is `"A"`.
    Character(InputText),
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Escape,
    Insert,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Shift,
    Control,
    Alt,
    /// The `AltGr` key, on layouts and platforms that report it as a key of its
    /// own (`ADR-0027`).
    AltGraph,
    /// The Windows / Command / Meta key.
    Super,
    CapsLock,
    /// A dead key waiting for the next press to compose with.
    Dead,
    /// A named key this port has no variant for.
    Other,
    /// A key the backend could not name at all.
    Unidentified,
}

impl fmt::Display for LogicalKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Character(text) => return fmt::Display::fmt(text, formatter),
            Self::Enter => "Enter",
            Self::Tab => "Tab",
            Self::Space => "Space",
            Self::Backspace => "Backspace",
            Self::Delete => "Delete",
            Self::Escape => "Escape",
            Self::Insert => "Insert",
            Self::ArrowUp => "ArrowUp",
            Self::ArrowDown => "ArrowDown",
            Self::ArrowLeft => "ArrowLeft",
            Self::ArrowRight => "ArrowRight",
            Self::Home => "Home",
            Self::End => "End",
            Self::PageUp => "PageUp",
            Self::PageDown => "PageDown",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
            Self::F11 => "F11",
            Self::F12 => "F12",
            Self::Shift => "Shift",
            Self::Control => "Control",
            Self::Alt => "Alt",
            Self::AltGraph => "AltGraph",
            Self::Super => "Super",
            Self::CapsLock => "CapsLock",
            Self::Dead => "Dead",
            Self::Other => "Other",
            Self::Unidentified => "Unidentified",
        };
        formatter.write_str(name)
    }
}
