//! What a browsing context has done so far.

/// A monotonically growing tally.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Counter(usize);

impl Counter {
    const fn increment(&mut self) {
        self.0 = self.0.saturating_add(1);
    }
}

/// One thing a context accepted, for [`ContextStats`] to count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accepted {
    /// A navigation parsed into the document tree.
    Navigation,
    /// A navigation failed and fell back to the error page.
    NavigationError,
    /// An external stylesheet joined the cascade.
    Stylesheet,
    /// An image decoded and replaced its placeholder.
    Image,
}

/// Counters of the documents and subresources a context accepted — the
/// executor folds them into its own loop statistics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ContextStats {
    navigations: Counter,
    navigation_errors: Counter,
    stylesheets: Counter,
    images: Counter,
}

impl ContextStats {
    /// Navigations that parsed into the document tree.
    #[must_use]
    pub const fn navigations(&self) -> usize {
        self.navigations.0
    }

    /// Navigations that failed and fell back to the error page.
    #[must_use]
    pub const fn navigation_errors(&self) -> usize {
        self.navigation_errors.0
    }

    /// External stylesheets absorbed into the cascade.
    #[must_use]
    pub const fn stylesheets_loaded(&self) -> usize {
        self.stylesheets.0
    }

    /// Images that decoded and replaced their placeholder.
    #[must_use]
    pub const fn images_loaded(&self) -> usize {
        self.images.0
    }

    pub(crate) const fn record(&mut self, accepted: Accepted) {
        match accepted {
            Accepted::Navigation => self.navigations.increment(),
            Accepted::NavigationError => self.navigation_errors.increment(),
            Accepted::Stylesheet => self.stylesheets.increment(),
            Accepted::Image => self.images.increment(),
        }
    }
}
