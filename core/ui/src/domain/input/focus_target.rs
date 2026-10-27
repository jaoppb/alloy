//! [`FocusTarget`] — who receives typing.

/// The part of the browser that receives resolved key input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusTarget {
    /// The address bar.
    Omnibox,
    /// The page. It has no keyboard consumer yet (form controls, #88), so key
    /// input aimed at it is dropped.
    Content,
}
