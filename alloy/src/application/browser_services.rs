//! The collaborators one browser session runs on.

use std::sync::Arc;

use graphics::FontProvider;
use network::{HttpTransport, RequestPolicy};

use crate::application::subresource::{MarkupDiscoverer, SubresourceDiscoverer};

/// Font, network and discovery collaborators of one browser session.
///
/// Bundled so the event loop stays statically dispatched (`F`, `T`, `P`, `D`
/// are all monomorphised — no `dyn`) without every function repeating four
/// arguments.
///
/// The transport and policy are shared by pointer because each background
/// fetch thread needs its own owning handle.
pub struct BrowserServices<F, T, P, D = MarkupDiscoverer> {
    font_provider: Arc<F>,
    transport: Arc<T>,
    policy: Arc<P>,
    discoverer: D,
}

impl<F: FontProvider, T: HttpTransport, P: RequestPolicy> BrowserServices<F, T, P> {
    /// Services discovering subresources from markup ([`MarkupDiscoverer`]).
    #[must_use]
    pub const fn new(font_provider: Arc<F>, transport: Arc<T>, policy: Arc<P>) -> Self {
        Self {
            font_provider,
            transport,
            policy,
            discoverer: MarkupDiscoverer,
        }
    }
}

impl<F, T, P, D> BrowserServices<F, T, P, D> {
    /// The same services discovering subresources through `discoverer`.
    #[must_use]
    pub fn with_discoverer<Other: SubresourceDiscoverer>(
        self,
        discoverer: Other,
    ) -> BrowserServices<F, T, P, Other> {
        BrowserServices {
            font_provider: self.font_provider,
            transport: self.transport,
            policy: self.policy,
            discoverer,
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

    /// Where a document's subresource references come from.
    #[must_use]
    pub const fn discoverer(&self) -> &D {
        &self.discoverer
    }
}
