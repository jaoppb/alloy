//! The browsing context: the document a window shows, as a pure state machine.
//!
//! Commands go in ([`BrowsingContext::navigate`], [`BrowsingContext::follow_link`],
//! [`BrowsingContext::complete`], [`BrowsingContext::layout_completed`]) and
//! [`BrowserEffect`]s come out. The context never spawns, sleeps or fetches
//! (#110 decision 4) — which is what lets every supersession rule below be
//! proved with plain function calls.
//!
//! [`BrowsingContext::complete`] and its helpers live in [`completion`].

use network::Url;

use crate::application::SubresourceDiscoverer;
use crate::domain::{
    BrowserEffect, ContentRegion, ContentRegions, ContextId, ContextStats, Document, Effects,
    FetchRequest, LayoutEpoch, LinkDestination, LinkTargets, NavigationId, Navigations, RegionId,
};

mod completion;
#[cfg(test)]
mod tests;

/// One browsing context — today always the root one (#110 decision 14).
pub struct BrowsingContext<D> {
    id: ContextId,
    navigations: Navigations,
    document: Option<Document>,
    regions: ContentRegions,
    discoverer: D,
    stats: ContextStats,
}

impl<D: SubresourceDiscoverer> BrowsingContext<D> {
    /// The root context, showing nothing yet, finding subresources with `discoverer`.
    #[must_use]
    pub fn new(discoverer: D) -> Self {
        let id = ContextId::ROOT;
        Self {
            id,
            navigations: Navigations::new(id),
            document: None,
            regions: ContentRegions::new(id),
            discoverer,
            stats: ContextStats::default(),
        }
    }

    /// Starts navigating to `url`, superseding every navigation and subresource
    /// fetch still in flight: their results will arrive stamped with an id that
    /// is no longer the latest and be dropped.
    pub fn navigate(&mut self, url: Url) -> Effects {
        self.navigations.supersede();
        let navigation = self.navigations.latest();
        Effects::from_iter([
            BrowserEffect::NavigationStarted(self.id, url.clone()),
            BrowserEffect::Fetch(FetchRequest::Document(url), navigation),
        ])
    }

    /// Follows the link `region` stands for. A region from an earlier layout is
    /// logged and ignored — never resolved against whatever now sits at its index.
    pub fn follow_link(&mut self, region: RegionId) -> Effects {
        match self.link_destination(region) {
            Some(LinkDestination::Navigate(target)) => {
                tracing::info!(url = %target, "link clicked, navigating");
                self.navigate(target)
            }
            Some(LinkDestination::InPageAnchor) => {
                tracing::info!("in-page anchor clicked (no-op in v0.5)");
                Effects::none()
            }
            None => Effects::none(),
        }
    }

    /// Takes the link areas of a finished layout and mints the [`RegionId`]s
    /// ([`BrowsingContext::content_regions`]) `core/ui` hit-tests, under a new epoch.
    pub fn layout_completed(&mut self, links: LinkTargets) -> LayoutEpoch {
        self.regions.replace(links);
        self.regions.epoch()
    }
}

impl<D> BrowsingContext<D> {
    /// The link regions of the last layout, in paint order.
    pub fn content_regions(&self) -> impl Iterator<Item = ContentRegion> + '_ {
        self.regions.iter()
    }

    /// The context's id.
    #[must_use]
    pub const fn id(&self) -> ContextId {
        self.id
    }

    /// The document shown now, if any navigation has completed.
    #[must_use]
    pub const fn document(&self) -> Option<&Document> {
        self.document.as_ref()
    }

    /// The latest navigation; results stamped with any other id are dropped.
    #[must_use]
    pub const fn current_navigation(&self) -> NavigationId {
        self.navigations.latest()
    }

    /// What the context has accepted so far.
    #[must_use]
    pub const fn stats(&self) -> ContextStats {
        self.stats
    }

    fn link_destination(&self, region: RegionId) -> Option<LinkDestination> {
        let href = self
            .regions
            .href(region)
            .inspect_err(|error| tracing::warn!(%error, "ignoring a click on a stale link region"))
            .ok()?;
        let base_url = self.document.as_ref()?.base_url()?;
        LinkDestination::resolve(base_url, href)
            .inspect_err(|error| tracing::warn!(href, %error, "failed to resolve link target"))
            .ok()
    }
}
