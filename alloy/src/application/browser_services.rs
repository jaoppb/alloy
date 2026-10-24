//! The collaborators one browser session runs on.

use std::sync::Arc;

use graphics::FontProvider;
use network::{HttpTransport, RequestPolicy};

/// Font and network collaborators of one browser session.
///
/// Bundled so the event loop stays statically dispatched (`F`, `T`, `P` are
/// all monomorphised — no `dyn`) without every function repeating three
/// arguments.
///
/// The transport and policy are shared by pointer because each background
/// fetch thread needs its own owning handle. Subresource discovery is not a
/// service of the session: the browsing context owns it (`core/browser`).
pub struct BrowserServices<F, T, P> {
    font_provider: Arc<F>,
    transport: Arc<T>,
    policy: Arc<P>,
}

impl<F: FontProvider, T: HttpTransport, P: RequestPolicy> BrowserServices<F, T, P> {
    /// Services over `font_provider`, `transport` and `policy`.
    #[must_use]
    pub const fn new(font_provider: Arc<F>, transport: Arc<T>, policy: Arc<P>) -> Self {
        Self {
            font_provider,
            transport,
            policy,
        }
    }

    /// The font used for layout measurement and painting.
    #[must_use]
    pub const fn font_provider(&self) -> &Arc<F> {
        &self.font_provider
    }

    /// The transport every fetch goes through.
    #[must_use]
    pub const fn transport(&self) -> &Arc<T> {
        &self.transport
    }

    /// The policy consulted before a navigation opens a connection.
    #[must_use]
    pub const fn policy(&self) -> &Arc<P> {
        &self.policy
    }
}
