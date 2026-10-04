//! Session state: the live document, its subresources, and the frame last
//! presented from them. How background-fetch results change that state lives
//! in [`messages`].

use std::sync::Arc;
use std::sync::mpsc::Sender;

use css::StyleSheetSet;
use dom::DomTree;
use graphics::FontProvider;
use network::{HttpTransport, RequestPolicy, Url};
use window::{PhysicalPosition, Presenter, SurfaceSize};

mod messages;

use super::frame::CachedFrame;
use super::hit_test::hit_test;
use super::stats::LoopStats;
use super::worker::{LoopMessage, spawn_navigation};
use crate::application::browser_services::BrowserServices;
use crate::application::image_store::ImageStore;
use crate::application::pipeline::{LinkTarget, render_dom_with_links};
use crate::application::subresource::SubresourceDiscoverer;
use crate::error::AlloyError;

/// Everything one pump cycle can mutate, bundled so `pump_once` stays under
/// the arity limit: the accumulated document state, the current viewport, and
/// the running relayout count.
///
/// Rebuilding a fresh display list from `dom_tree`/`extra_sheets`/`images` on
/// every relayout is the same "immutable snapshot in, immutable snapshot out"
/// discipline the render pipeline itself uses (`ADR-0010:114-117`), applied
/// to the state a live session must keep between frames.
///
/// Every field is private: `dirty` → [`Session::relayout`] → clean, and
/// `links` always describing `last_frame`, are invariants only this type's
/// methods may move.
pub struct Session<F, T, P, D> {
    services: BrowserServices<F, T, P, D>,
    dom_tree: Option<DomTree>,
    /// The document's effective base URL — `<base href>` already applied —
    /// that both subresource discovery and link clicks resolve against.
    base_url: Option<Url>,
    extra_sheets: StyleSheetSet,
    images: ImageStore,
    /// The link areas of `last_frame`, in paint order.
    links: Vec<LinkTarget>,
    pointer_position: Option<PhysicalPosition>,
    dirty: bool,
    viewport: SurfaceSize,
    last_frame: Option<CachedFrame>,
    stats: LoopStats,
}

impl<F, T, P, D> Session<F, T, P, D>
where
    F: FontProvider + 'static,
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
    D: SubresourceDiscoverer,
{
    pub fn new(viewport: SurfaceSize, services: BrowserServices<F, T, P, D>) -> Self {
        Self {
            services,
            dom_tree: None,
            base_url: None,
            extra_sheets: StyleSheetSet::new(),
            images: ImageStore::new(),
            links: Vec::new(),
            pointer_position: None,
            dirty: false,
            viewport,
            last_frame: None,
            stats: LoopStats::default(),
        }
    }

    pub const fn stats(&self) -> LoopStats {
        self.stats
    }

    pub const fn pointer_position(&self) -> Option<PhysicalPosition> {
        self.pointer_position
    }

    /// Where the pointer was last seen — `None` until it first enters.
    pub const fn track_pointer(&mut self, position: Option<PhysicalPosition>) {
        self.pointer_position = position;
    }

    /// Adopts the new viewport; the next [`Session::relayout`] lays out at it.
    pub const fn resize(&mut self, viewport: SurfaceSize) {
        self.viewport = viewport;
        self.dirty = true;
    }

    pub const fn needs_relayout(&self) -> bool {
        self.dirty
    }

    /// Starts fetching `url` on a worker thread; the result arrives as a
    /// [`LoopMessage::Navigation`].
    pub fn navigate(&self, url: Url, sender: &Sender<LoopMessage>) {
        let transport = Arc::clone(self.services.transport());
        let policy = Arc::clone(self.services.policy());
        spawn_navigation(url, transport, policy, sender.clone());
    }

    /// Navigates to the topmost link under `position`, resolved against the
    /// document's base URL. An in-page `#anchor` is a no-op in v0.5.
    pub fn follow_link_at(&self, position: PhysicalPosition, sender: &Sender<LoopMessage>) {
        let Some(href) = hit_test(&self.links, position) else {
            return;
        };
        let Some(base_url) = self.base_url.as_ref() else {
            return;
        };
        if href.starts_with('#') {
            tracing::info!(anchor = href, "in-page anchor clicked (no-op in v0.5)");
            return;
        }
        match base_url.join(href) {
            Ok(target_url) => {
                tracing::info!(url = %target_url, "link clicked, navigating");
                self.navigate(target_url, sender);
            }
            Err(error) => tracing::warn!(href, %error, "failed to resolve link target"),
        }
    }

    /// Rebuilds the display list from the current document, viewport and
    /// subresources, presents it, and caches the pixels for a later cheap
    /// [`Session::repaint`]. The only path that bumps `stats.relayouts`, and
    /// only once a frame was actually presented — the I4 coalescing proof
    /// and `run_browser_until_first_frame` both rest on that. With no
    /// document yet there is nothing to lay out, and the request is dropped.
    pub fn relayout<R: Presenter>(&mut self, presenter: &mut R) -> Result<(), AlloyError> {
        self.dirty = false;
        let Some(dom_tree) = self.dom_tree.as_ref() else {
            return Ok(());
        };
        let graphics_size =
            graphics::SurfaceSize::new(self.viewport.width(), self.viewport.height())
                .ok_or(AlloyError::InvalidDimensions)?;
        let (framebuffer, links) = render_dom_with_links(
            dom_tree,
            self.extra_sheets.clone(),
            &self.images,
            graphics_size,
            Arc::clone(self.services.font_provider()),
        )?;
        let frame = CachedFrame::capture(&framebuffer, self.viewport);
        frame.present(presenter)?;
        self.links = links;
        self.last_frame = Some(frame);
        self.stats.relayouts = self.stats.relayouts.saturating_add(1);
        Ok(())
    }

    /// Re-blits the frame [`Session::relayout`] last produced, with no
    /// pipeline work and without touching `stats.relayouts`. A no-op before
    /// the first frame. Serves `RedrawRequested`: a compositor
    /// expose/occlusion, or the redraw winit re-arms once a Wayland surface
    /// that dropped the first present is finally configured.
    pub fn repaint<R: Presenter>(&self, presenter: &mut R) -> Result<(), AlloyError> {
        let Some(frame) = self.last_frame.as_ref() else {
            return Ok(());
        };
        frame.present(presenter)
    }
}

/// Read-only views the `pump_once` tests assert on.
#[cfg(test)]
impl<F, T, P, D> Session<F, T, P, D> {
    pub const fn viewport(&self) -> SurfaceSize {
        self.viewport
    }

    pub const fn base_url(&self) -> Option<&Url> {
        self.base_url.as_ref()
    }

    pub const fn has_frame(&self) -> bool {
        self.last_frame.is_some()
    }

    pub const fn has_links(&self) -> bool {
        !self.links.is_empty()
    }
}
