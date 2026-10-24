//! The collaborators one browser session runs on.

use std::sync::Arc;

use crate::application::fetch_channel::FetchInbox;

/// Font, fetching and pacing collaborators of one browser session.
///
/// Bundled so the event loop stays statically dispatched (`F`, `X` and `W` are
/// monomorphised — no `dyn`) without every function repeating the arguments.
///
/// The executor and the inbox are a pair: results the executor finishes are
/// the ones the session drains. Subresource discovery is not a service of the
/// session: the browsing context owns it (`core/browser`).
pub struct BrowserServices<F, X, W> {
    font_provider: Arc<F>,
    fetch_executor: X,
    fetch_inbox: FetchInbox,
    idle_pause: W,
}

impl<F, X, W> BrowserServices<F, X, W> {
    /// Services over `font_provider`, the `fetch_executor` whose results
    /// arrive in `fetch_inbox`, and the `idle_pause` an idle loop waits with.
    #[must_use]
    pub const fn new(
        font_provider: Arc<F>,
        fetch_executor: X,
        fetch_inbox: FetchInbox,
        idle_pause: W,
    ) -> Self {
        Self {
            font_provider,
            fetch_executor,
            fetch_inbox,
            idle_pause,
        }
    }

    /// The font used for layout measurement and painting.
    #[must_use]
    pub const fn font_provider(&self) -> &Arc<F> {
        &self.font_provider
    }

    /// Runs every fetch the browsing context asks for.
    #[must_use]
    pub const fn fetch_executor(&self) -> &X {
        &self.fetch_executor
    }

    /// Where the executor's finished fetches wait for the loop.
    #[must_use]
    pub const fn fetch_inbox(&self) -> &FetchInbox {
        &self.fetch_inbox
    }

    /// What an idle pump cycle waits with.
    #[must_use]
    pub const fn idle_pause(&self) -> &W {
        &self.idle_pause
    }
}
