//! Which navigation a background-fetch result belongs to.

/// Counts the navigations a [`super::session::Session`] has started.
///
/// A worker thread cannot be cancelled once spawned, so every result it sends
/// carries the generation it was started under and the session drops any
/// result whose generation is no longer current. This is also what keeps an
/// `ImageId` — a DOM node index, reused by every document — from naming an
/// `<img>` of a page the user already left.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NavigationGeneration(u64);

impl NavigationGeneration {
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}
