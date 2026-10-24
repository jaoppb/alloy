//! Which navigation is current, and the ids that identify one.
//!
//! A fetch cannot be cancelled, so a result is *stamped* with the
//! [`NavigationId`] it was started for and dropped on arrival when that is no
//! longer the latest one (ADR-0019 amendment, #111). This replaces `e832b6a`'s
//! channel-per-navigation, trading a comparison for a surface that needs no
//! thread and no sleep to test.

use std::fmt;

use crate::domain::ContextId;

/// One navigation of one browsing context. A document, and every subresource
/// fetched for it, report under the same id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NavigationId {
    context: ContextId,
    sequence: u64,
}

impl NavigationId {
    /// The context this navigation belongs to.
    #[must_use]
    pub const fn context(self) -> ContextId {
        self.context
    }
}

impl fmt::Display for NavigationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/navigation#{}", self.context, self.sequence)
    }
}

/// Tracks the latest [`NavigationId`] a context has issued.
///
/// Starts on a *genesis* id that no `navigate` has issued yet: nothing is in
/// flight, so nothing is stamped with it, and it keeps [`Navigations::latest`]
/// total.
#[derive(Clone, Copy, Debug)]
pub struct Navigations {
    latest: NavigationId,
}

impl Navigations {
    /// A tracker for `context` that has navigated nowhere yet.
    #[must_use]
    pub const fn new(context: ContextId) -> Self {
        Self {
            latest: NavigationId {
                context,
                sequence: 0,
            },
        }
    }

    /// Makes a fresh id the latest one, superseding everything in flight.
    pub const fn supersede(&mut self) {
        self.latest.sequence = self.latest.sequence.saturating_add(1);
    }

    /// The id results are accepted under.
    #[must_use]
    pub const fn latest(&self) -> NavigationId {
        self.latest
    }

    /// Whether a result stamped with `id` still belongs to the page the user
    /// is on (or is heading to).
    #[must_use]
    pub fn is_latest(&self, id: NavigationId) -> bool {
        self.latest == id
    }
}

#[cfg(test)]
mod tests {
    use super::Navigations;
    use crate::domain::ContextId;

    #[test]
    fn superseding_makes_the_previous_id_stale() {
        let mut navigations = Navigations::new(ContextId::ROOT);
        navigations.supersede();
        let first = navigations.latest();
        navigations.supersede();

        assert!(
            !navigations.is_latest(first),
            "the earlier id is superseded"
        );
        assert!(navigations.is_latest(navigations.latest()));
    }

    #[test]
    fn every_supersession_issues_a_distinct_id() {
        let mut navigations = Navigations::new(ContextId::ROOT);
        navigations.supersede();
        let first = navigations.latest();
        navigations.supersede();

        assert_ne!(first, navigations.latest());
    }

    #[test]
    fn an_id_carries_the_context_it_was_issued_in() {
        let mut navigations = Navigations::new(ContextId::ROOT);
        navigations.supersede();

        assert_eq!(navigations.latest().context(), ContextId::ROOT);
    }
}
