//! [`CursorIcon`] — the pointer image
//! [`WindowSystem::set_cursor`](crate::application::ports::WindowSystem::set_cursor)
//! asks for.

/// A pointer image, named by what it tells the user rather than by how a
/// platform draws it.
///
/// A closed, small vocabulary: the shapes a browser actually asks for (link
/// hover, text field, busy). `#[non_exhaustive]` so a later shape (resize
/// handles, drag-and-drop) is additive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CursorIcon {
    /// The platform's arrow.
    #[default]
    Default,
    /// Over something activatable, e.g. a link.
    Pointer,
    /// Over editable or selectable text.
    Text,
    /// The application is busy and cannot take input.
    Wait,
    /// The application is working but still takes input.
    Progress,
    Crosshair,
    /// The action under the pointer is unavailable.
    NotAllowed,
}
