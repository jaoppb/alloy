//! [`FocusTarget`] — who receives typing.

/// The part of the browser that receives resolved key input.
///
/// Today it is a two-state view of whether the omnibox holds focus (the
/// controller derives it, so the two cannot disagree). When the page gains a
/// keyboard consumer (#88) focus becomes real state and that derivation is
/// the one thing to replace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusTarget {
    /// The address bar.
    Omnibox,
    /// The page. It has no keyboard consumer yet (form controls, #88), so key
    /// input aimed at it is dropped.
    Content,
}
