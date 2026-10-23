//! What a browsing context has done so far.

/// Counters of the documents and subresources a context accepted — the
/// executor folds them into its own loop statistics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ContextStats {
    navigations: usize,
    navigation_errors: usize,
    stylesheets_loaded: usize,
    images_loaded: usize,
}

impl ContextStats {
    /// Navigations that parsed into the document tree.
    #[must_use]
    pub const fn navigations(&self) -> usize {
        self.navigations
    }

    /// Navigations that failed and fell back to the error page.
    #[must_use]
    pub const fn navigation_errors(&self) -> usize {
        self.navigation_errors
    }

    /// External stylesheets absorbed into the cascade.
    #[must_use]
    pub const fn stylesheets_loaded(&self) -> usize {
        self.stylesheets_loaded
    }

    /// Images that decoded and replaced their placeholder.
    #[must_use]
    pub const fn images_loaded(&self) -> usize {
        self.images_loaded
    }

    pub(crate) const fn count_navigation(&mut self) {
        self.navigations = self.navigations.saturating_add(1);
    }

    pub(crate) const fn count_navigation_error(&mut self) {
        self.navigation_errors = self.navigation_errors.saturating_add(1);
    }

    pub(crate) const fn count_stylesheet(&mut self) {
        self.stylesheets_loaded = self.stylesheets_loaded.saturating_add(1);
    }

    pub(crate) const fn count_image(&mut self) {
        self.images_loaded = self.images_loaded.saturating_add(1);
    }
}
