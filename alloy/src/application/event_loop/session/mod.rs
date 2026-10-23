//! Session state: the live document, its subresources, and the frame last
//! presented from them. How background-fetch results change that state lives
//! in [`messages`].

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use css::StyleSheetSet;
use dom::DomTree;
use graphics::{DisplayListBuilder, FontProvider, RenderBackend, SoftwareCpuBackend};
use network::{HttpTransport, RequestPolicy, Url};
use window::{PhysicalPosition, Presenter, SurfaceSize};

mod messages;

use super::frame::CachedFrame;
use super::render_tree::{UiHitTarget, UiRenderTree};
use super::stats::LoopStats;
use super::worker::{LoopMessage, spawn_navigation};
use crate::application::browser_services::BrowserServices;
use crate::application::image_store::ImageStore;
use crate::application::omnibox::{OmniboxController, OmniboxEffect, OmniboxPainter, OmniboxState};
use crate::application::pipeline::{LinkTarget, render_dom_with_links, surface_length_to_au};
use crate::application::subresource::SubresourceDiscoverer;
use crate::application::theme::{OmniboxTheme, Theme};
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
    omnibox: OmniboxController,
    /// Where the workers of the current navigation report. A worker cannot be
    /// cancelled, so [`Session::open_channel`] replaces both ends on every
    /// navigation: dropping the old `inbox` discards what its workers already
    /// queued and makes their later `send` fail, so a result from a page the
    /// user left has no way back in. That includes an `ImageId` — a DOM node
    /// index reused by every document — naming an `<img>` of the new page.
    inbox: Receiver<LoopMessage>,
    outbox: Sender<LoopMessage>,
    /// The one source of chrome geometry: the chrome is painted with it and
    /// `ui_tree` is laid out from it, so clicks land where pixels are drawn.
    theme: Theme,
    ui_tree: UiRenderTree,
}

impl<F, T, P, D> Session<F, T, P, D>
where
    F: FontProvider + 'static,
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
    D: SubresourceDiscoverer,
{
    pub fn new(viewport: SurfaceSize, services: BrowserServices<F, T, P, D>) -> Self {
        let (outbox, inbox) = mpsc::channel();
        let theme = Theme::DEFAULT;
        let ui_tree = UiRenderTree::build(viewport, theme.omnibox(), &[])
            .unwrap_or_else(|_| UiRenderTree::fallback(viewport, theme.omnibox()));
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
            omnibox: OmniboxController::new(),
            inbox,
            outbox,
            theme,
            ui_tree,
        }
    }

    pub const fn stats(&self) -> LoopStats {
        self.stats
    }

    /// Mutable access to the session's omnibox controller.
    pub const fn omnibox_mut(&mut self) -> &mut OmniboxController {
        &mut self.omnibox
    }

    /// Executes an effect emitted by the omnibox controller.
    ///
    /// Returns `true` if a platform redraw is requested.
    pub fn execute_omnibox_effect(&mut self, effect: OmniboxEffect) -> bool {
        match effect {
            OmniboxEffect::None => false,
            OmniboxEffect::Redraw => true,
            OmniboxEffect::Navigate(url) => {
                self.navigate(url);
                true
            }
        }
    }

    pub const fn pointer_position(&self) -> Option<PhysicalPosition> {
        self.pointer_position
    }

    /// Read-only access to the session's UI render tree.
    #[must_use]
    #[cfg(test)]
    pub const fn ui_tree(&self) -> &UiRenderTree {
        &self.ui_tree
    }

    /// Where the pointer was last seen — `None` until it first enters.
    pub const fn track_pointer(&mut self, position: Option<PhysicalPosition>) {
        self.pointer_position = position;
    }

    /// Adopts the new viewport; the next [`Session::relayout`] lays out at it.
    pub fn resize(&mut self, viewport: SurfaceSize) {
        self.viewport = viewport;
        self.ui_tree = UiRenderTree::build(viewport, self.theme.omnibox(), &self.links)
            .unwrap_or_else(|_| UiRenderTree::fallback(viewport, self.theme.omnibox()));
        self.dirty = true;
    }

    pub const fn needs_relayout(&self) -> bool {
        self.dirty
    }

    /// Starts fetching `url` on a worker thread; the result arrives as a
    /// [`LoopMessage::Navigation`]. Supersedes every navigation and
    /// subresource fetch still in flight: they report on a channel nobody
    /// reads any more.
    pub fn navigate(&mut self, url: Url) {
        self.open_channel();
        self.omnibox.reset_to_url(&url);
        let transport = Arc::clone(self.services.transport());
        let policy = Arc::clone(self.services.policy());
        spawn_navigation(url, transport, policy, self.outbox.clone());
    }

    /// Dispatches a pointer click at `position` through the UI render tree:
    /// - Clicks on the omnibox or chrome top bar focus the omnibox.
    /// - Clicks on a hyperlink navigate to the resolved destination.
    /// - Clicks on empty page content do nothing.
    pub fn follow_link_at(&mut self, position: PhysicalPosition) {
        let Some(target) = self.ui_tree.hit_test(position) else {
            return;
        };
        match target {
            UiHitTarget::Omnibox | UiHitTarget::Chrome => {
                self.omnibox.focus();
            }
            UiHitTarget::PageLink(href) => {
                let href = href.to_owned();
                self.follow_link(&href);
            }
            UiHitTarget::PageContent => {}
        }
    }

    fn follow_link(&mut self, href: &str) {
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
                self.navigate(target_url);
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
        let graphics_size = self.ui_tree.content_size()?;
        let (content_framebuffer, links) = render_dom_with_links(
            dom_tree,
            self.extra_sheets.clone(),
            &self.images,
            graphics_size,
            Arc::clone(self.services.font_provider()),
        )?;
        let chrome_framebuffer = render_chrome(
            self.omnibox.state(),
            self.theme.omnibox(),
            self.ui_tree.chrome_size()?,
            Arc::clone(self.services.font_provider()),
        )?;
        let frame = CachedFrame::compose(&chrome_framebuffer, &content_framebuffer, self.viewport)?;
        frame.present(presenter)?;
        self.ui_tree = UiRenderTree::build(self.viewport, self.theme.omnibox(), &links)?;
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

impl<F, T, P, D> Session<F, T, P, D> {
    fn open_channel(&mut self) {
        (self.outbox, self.inbox) = mpsc::channel();
    }
}

fn render_chrome<F: FontProvider + 'static>(
    omnibox: &OmniboxState,
    theme: &OmniboxTheme,
    chrome_size: graphics::SurfaceSize,
    font_provider: Arc<F>,
) -> Result<graphics::Framebuffer, AlloyError> {
    let mut builder = DisplayListBuilder::new();
    let painter = OmniboxPainter::new(font_provider.as_ref()).with_theme(*theme);
    painter.paint(
        omnibox,
        surface_length_to_au(chrome_size.width())?.to_px(),
        &mut builder,
    )?;
    let display_list = builder.build()?;

    let mut backend = SoftwareCpuBackend::with_font_provider(font_provider);
    backend.begin_frame(chrome_size)?;
    backend.submit(&display_list)?;
    backend.end_frame()?;
    let framebuffer = backend.read_back()?;
    Ok(framebuffer)
}

/// Read-only views the `pump_once` tests assert on.
#[cfg(test)]
impl<F, T, P, D> Session<F, T, P, D> {
    pub const fn omnibox(&self) -> &OmniboxController {
        &self.omnibox
    }

    /// A clone of the sender the current page's workers hold.
    pub fn outbox_for_test(&self) -> Sender<LoopMessage> {
        self.outbox.clone()
    }

    /// Supersedes the current page the way [`Session::navigate`] does, without
    /// spawning a fetch.
    pub fn begin_navigation_for_test(&mut self) {
        self.open_channel();
    }

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
