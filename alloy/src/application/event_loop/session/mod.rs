//! Session state: the browsing context, the chrome around it, and the frame
//! last presented from them. The document itself — and the rules for which
//! fetch results may change it — live in `core/browser`; this type executes the
//! [`BrowserEffect`]s it returns.

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use browser::{
    BrowserEffect, BrowsingContext, Effects, FetchResult, LinkTargets, MarkupDiscoverer,
};
use graphics::{DisplayListBuilder, FontProvider, RenderBackend, SoftwareCpuBackend};
use network::{HttpTransport, RequestPolicy, Url};
use region::ContentRegion;
use ui::{
    OmniboxController, OmniboxEffect, OmniboxPainter, OmniboxState, OmniboxTheme, Theme,
    UiHitTarget, UiRenderTree,
};
use window::{PhysicalPosition, Presenter, SurfaceSize};

use super::frame::CachedFrame;
use super::stats::LoopStats;
use super::worker::spawn_fetch;
use crate::application::browser_services::BrowserServices;
use crate::application::paint::DEFAULT_FONT;
use crate::application::pipeline::{render_dom_with_links, surface_length_to_au};
use crate::error::AlloyError;

/// Everything one pump cycle can mutate, bundled so `pump_once` stays under
/// the arity limit: the browsing context, the current viewport, and the
/// running relayout count.
///
/// Rebuilding a fresh display list from the context's document on every
/// relayout is the same "immutable snapshot in, immutable snapshot out"
/// discipline the render pipeline itself uses (`ADR-0010:114-117`), applied
/// to the state a live session must keep between frames.
///
/// Every field is private: `dirty` → [`Session::relayout`] → clean, and the
/// context's link regions always describing `last_frame`, are invariants only
/// this type's methods may move.
pub struct Session<F, T, P> {
    services: BrowserServices<F, T, P>,
    browsing: BrowsingContext<MarkupDiscoverer>,
    pointer_position: Option<PhysicalPosition>,
    dirty: bool,
    viewport: SurfaceSize,
    last_frame: Option<CachedFrame>,
    relayouts: usize,
    omnibox: OmniboxController,
    /// Where the fetch workers report. One channel for the session's life: a
    /// result from a page the user left is not cut off here but dropped by the
    /// browsing context, because it is stamped with a superseded
    /// `NavigationId` (ADR-0019 amendment).
    inbox: Receiver<FetchResult>,
    outbox: Sender<FetchResult>,
    /// The one source of chrome geometry: the chrome is painted with it and
    /// `ui_tree` is laid out from it, so clicks land where pixels are drawn.
    theme: Theme,
    ui_tree: UiRenderTree,
}

impl<F, T, P> Session<F, T, P>
where
    F: FontProvider + 'static,
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
{
    pub fn new(viewport: SurfaceSize, services: BrowserServices<F, T, P>) -> Self {
        let (outbox, inbox) = mpsc::channel();
        let theme = Theme::DEFAULT;
        let ui_tree = ui_tree_for(viewport, &theme, []);
        Self {
            services,
            browsing: BrowsingContext::new(MarkupDiscoverer),
            pointer_position: None,
            dirty: false,
            viewport,
            last_frame: None,
            relayouts: 0,
            omnibox: OmniboxController::new(),
            inbox,
            outbox,
            theme,
            ui_tree,
        }
    }

    pub const fn stats(&self) -> LoopStats {
        let browsed = self.browsing.stats();
        LoopStats {
            relayouts: self.relayouts,
            navigations: browsed.navigations(),
            navigation_errors: browsed.navigation_errors(),
            stylesheets_loaded: browsed.stylesheets_loaded(),
            images_loaded: browsed.images_loaded(),
        }
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
        self.ui_tree = ui_tree_for(viewport, &self.theme, self.browsing.content_regions());
        self.dirty = true;
    }

    pub const fn needs_relayout(&self) -> bool {
        self.dirty
    }

    /// Starts navigating to `url`; the document arrives later as a
    /// [`FetchResult`]. Supersedes every navigation and subresource fetch still
    /// in flight: their results are dropped by the browsing context.
    pub fn navigate(&mut self, url: Url) {
        let effects = self.browsing.navigate(url);
        self.execute_browser_effects(effects);
    }

    /// Applies every fetch result waiting now; tells the caller whether there
    /// were any.
    pub fn drain_messages(&mut self) -> bool {
        let mut saw_message = false;
        while let Ok(result) = self.inbox.try_recv() {
            saw_message = true;
            let effects = self.browsing.complete(result);
            self.execute_browser_effects(effects);
        }
        saw_message
    }

    /// Carries out what the browsing context asked for: fetches run on worker
    /// threads, the omnibox follows the URL, damage schedules a relayout.
    fn execute_browser_effects(&mut self, effects: Effects) {
        for effect in effects {
            match effect {
                BrowserEffect::NavigationStarted(_, url)
                | BrowserEffect::DocumentLoaded(_, url) => {
                    self.omnibox.reset_to_url(&url);
                }
                BrowserEffect::Fetch(request, navigation) => {
                    let transport = Arc::clone(self.services.transport());
                    let policy = Arc::clone(self.services.policy());
                    spawn_fetch(request, navigation, transport, policy, self.outbox.clone());
                }
                BrowserEffect::ContentDamaged(_) => self.dirty = true,
            }
        }
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
            UiHitTarget::PageLink(region) => {
                let effects = self.browsing.follow_link(region);
                self.execute_browser_effects(effects);
            }
            UiHitTarget::PageContent => {}
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
        let Some(document) = self.browsing.document() else {
            return Ok(());
        };
        let graphics_size = self.ui_tree.content_size()?;
        let (content_framebuffer, links) = render_dom_with_links(
            document.tree(),
            document.sheets().clone(),
            document.images(),
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
        self.commit_layout(links, frame);
        Ok(())
    }

    /// Adopts a presented layout in one step, so the link regions, the UI tree
    /// hit-testing them and the cached frame never describe different layouts.
    /// Nothing here can fail: everything fallible ran before the frame was shown.
    fn commit_layout(&mut self, links: LinkTargets, frame: CachedFrame) {
        self.browsing.layout_completed(links);
        self.ui_tree = ui_tree_for(self.viewport, &self.theme, self.browsing.content_regions());
        self.last_frame = Some(frame);
        self.relayouts = self.relayouts.saturating_add(1);
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

/// The UI tree for `viewport`, or the inert fallback when the viewport cannot
/// hold the chrome (#118 tracks making that a visible state).
fn ui_tree_for(
    viewport: SurfaceSize,
    theme: &Theme,
    regions: impl IntoIterator<Item = ContentRegion>,
) -> UiRenderTree {
    UiRenderTree::build(viewport, theme.omnibox(), regions)
        .unwrap_or_else(|_| UiRenderTree::fallback(viewport, theme.omnibox()))
}

fn render_chrome<F: FontProvider + 'static>(
    omnibox: &OmniboxState,
    theme: &OmniboxTheme,
    chrome_size: graphics::SurfaceSize,
    font_provider: Arc<F>,
) -> Result<graphics::Framebuffer, AlloyError> {
    let mut builder = DisplayListBuilder::new();
    let painter = OmniboxPainter::new(font_provider.as_ref())
        .with_theme(*theme)
        .with_font(DEFAULT_FONT);
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
impl<F, T, P> Session<F, T, P> {
    pub const fn omnibox(&self) -> &OmniboxController {
        &self.omnibox
    }

    /// A clone of the sender the fetch workers hold.
    pub fn outbox_for_test(&self) -> Sender<FetchResult> {
        self.outbox.clone()
    }

    /// The navigation results are accepted under right now.
    pub const fn current_navigation_for_test(&self) -> browser::NavigationId {
        self.browsing.current_navigation()
    }

    pub const fn viewport(&self) -> SurfaceSize {
        self.viewport
    }

    pub fn base_url(&self) -> Option<&Url> {
        self.browsing
            .document()
            .and_then(browser::Document::base_url)
    }

    pub const fn has_frame(&self) -> bool {
        self.last_frame.is_some()
    }

    pub fn has_links(&self) -> bool {
        self.browsing.content_regions().next().is_some()
    }
}
