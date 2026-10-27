//! Session state: the browsing context, the chrome around it, and the frame
//! last presented from them. The document itself — and the rules for which
//! fetch results may change it — live in `core/browser`; this type executes the
//! [`BrowserEffect`]s it returns.

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use browser::{
    BrowserEffect, BrowsingContext, Effects, FetchResult, LinkTargets, MarkupDiscoverer,
};
use graphics::{FontProvider, RenderBackend};
use network::{HttpTransport, RequestPolicy, Url};
use region::ContentRegion;
use ui::{OmniboxController, OmniboxEffect, Theme, UiHitTarget, UiRenderTree};
use window::{PhysicalPosition, Presenter, SurfaceSize};

use super::stats::LoopStats;
use super::worker::spawn_fetch;
use crate::application::browser_services::BrowserServices;
use crate::application::compositor::{
    ChromeScene, Compositor, ContentScene, Damage, Layout, PendingDamage, Presented, Rendered,
    Scene,
};
use crate::error::AlloyError;

/// Everything one pump cycle can mutate, bundled so `pump_once` stays under
/// the arity limit: the browsing context, the current viewport, the damage
/// waiting for the compositor, and the running relayout count.
///
/// Rebuilding a fresh display list from the context's document on every
/// relayout is the same "immutable snapshot in, immutable snapshot out"
/// discipline the render pipeline itself uses (`ADR-0010:114-117`), applied
/// to the state a live session must keep between frames.
///
/// Every field is private: `damage` → [`Session::render`] → clean, and the
/// context's link regions always describing the content layer last rastered,
/// are invariants only this type's methods may move.
pub struct Session<F, T, P, B> {
    services: BrowserServices<F, T, P>,
    browsing: BrowsingContext<MarkupDiscoverer>,
    pointer_position: Option<PhysicalPosition>,
    damage: PendingDamage,
    viewport: SurfaceSize,
    compositor: Compositor<B>,
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

impl<F, T, P, B> Session<F, T, P, B>
where
    F: FontProvider + 'static,
    T: HttpTransport + 'static,
    P: RequestPolicy + 'static,
    B: RenderBackend,
{
    pub fn new(
        viewport: SurfaceSize,
        services: BrowserServices<F, T, P>,
        compositor: Compositor<B>,
    ) -> Self {
        let (outbox, inbox) = mpsc::channel();
        let theme = Theme::DEFAULT;
        let ui_tree = ui_tree_for(viewport, &theme, []);
        let mut damage = PendingDamage::default();
        // The chrome has never been rastered: the first frame needs it.
        damage.record(Damage::Chrome);
        Self {
            services,
            browsing: BrowsingContext::new(MarkupDiscoverer),
            pointer_position: None,
            damage,
            viewport,
            compositor,
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

    /// Executes an effect emitted by the omnibox controller: a redraw is
    /// chrome damage, never a relayout of the document.
    pub fn execute_omnibox_effect(&mut self, effect: OmniboxEffect) {
        match effect {
            OmniboxEffect::None => {}
            OmniboxEffect::Redraw => self.damage.record(Damage::Chrome),
            OmniboxEffect::Navigate(url) => self.navigate(url),
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

    /// Adopts the new viewport; the next [`Session::render`] sizes both layers
    /// to it.
    pub fn resize(&mut self, viewport: SurfaceSize) {
        self.viewport = viewport;
        self.ui_tree = ui_tree_for(viewport, &self.theme, self.browsing.content_regions());
        self.damage.record(Damage::Viewport);
    }

    #[cfg(test)]
    pub const fn has_pending_damage(&self) -> bool {
        !self.damage.is_empty()
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
    /// threads, the omnibox follows the URL (chrome damage), document damage
    /// schedules a relayout.
    fn execute_browser_effects(&mut self, effects: Effects) {
        for effect in effects {
            match effect {
                BrowserEffect::NavigationStarted(_, url)
                | BrowserEffect::DocumentLoaded(_, url) => {
                    self.omnibox.reset_to_url(&url);
                    self.damage.record(Damage::Chrome);
                }
                BrowserEffect::Fetch(request, navigation) => {
                    let transport = Arc::clone(self.services.transport());
                    let policy = Arc::clone(self.services.policy());
                    spawn_fetch(request, navigation, transport, policy, self.outbox.clone());
                }
                BrowserEffect::ContentDamaged(_) => self.damage.record(Damage::Content),
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
                self.damage.record(Damage::Chrome);
            }
            UiHitTarget::PageLink(region) => {
                let effects = self.browsing.follow_link(region);
                self.execute_browser_effects(effects);
            }
            UiHitTarget::PageContent => {}
        }
    }

    /// Rasters whatever the accumulated damage invalidated, composes the
    /// layers and presents once.
    ///
    /// Chrome damage re-rasters the chrome strip only. Content damage is the
    /// relayout — the only path that bumps `stats.relayouts`, and only once a
    /// frame laid out from a document was actually presented (the I4
    /// coalescing proof and `run_browser_until_first_frame` both rest on that).
    /// With no document yet there is nothing to lay out, and that damage is
    /// dropped. A render that fails puts its damage back, so nothing it
    /// consumed is lost.
    pub fn render<R: Presenter>(&mut self, presenter: &mut R) -> Result<Presented, AlloyError> {
        let damage = self.damage.take();
        let rendered = match self.render_damage(damage, presenter) {
            Ok(rendered) => rendered,
            Err(error) => {
                self.damage.restore(damage);
                return Err(error);
            }
        };
        let shown = rendered.presented();
        self.commit_layout(rendered.into_layout(), shown);
        Ok(shown)
    }

    fn render_damage<R: Presenter>(
        &mut self,
        damage: PendingDamage,
        presenter: &mut R,
    ) -> Result<Rendered, AlloyError> {
        let scene = Scene::new(
            ChromeScene::new(
                self.omnibox.state(),
                self.theme.omnibox(),
                self.ui_tree.chrome_size()?,
            ),
            ContentScene::new(self.browsing.document(), self.ui_tree.content_size()?),
            self.services.font_provider(),
            self.viewport,
        );
        self.compositor.render(damage, &scene, presenter)
    }

    /// Adopts a rastered layout in one step, so the link regions and the UI
    /// tree hit-testing them never describe a different layout than the
    /// content layer. Nothing here can fail: everything fallible ran before the
    /// frame was shown.
    fn commit_layout(&mut self, layout: Layout, shown: Presented) {
        let links = match layout {
            Layout::Untouched => return,
            Layout::NoRows => LinkTargets::default(),
            Layout::Done(links) => {
                self.count_relayout(shown);
                links
            }
        };
        self.browsing.layout_completed(links);
        self.ui_tree = ui_tree_for(self.viewport, &self.theme, self.browsing.content_regions());
    }

    const fn count_relayout(&mut self, shown: Presented) {
        if shown.is_shown() {
            self.relayouts = self.relayouts.saturating_add(1);
        }
    }

    /// Presents the last composed frame again, with no raster work and without
    /// touching `stats.relayouts`.
    pub fn repaint<R: Presenter>(&self, presenter: &mut R) -> Result<(), AlloyError> {
        self.compositor.repaint(presenter)
    }
}

/// The UI tree for `viewport`, or the inert fallback when the viewport cannot
/// hold the chrome; the compositor then shows the chrome alone (#118).
fn ui_tree_for(
    viewport: SurfaceSize,
    theme: &Theme,
    regions: impl IntoIterator<Item = ContentRegion>,
) -> UiRenderTree {
    UiRenderTree::build(viewport, theme.omnibox(), regions)
        .unwrap_or_else(|_| UiRenderTree::fallback(viewport, theme.omnibox()))
}

/// Read-only views the `pump_once` tests assert on.
#[cfg(test)]
impl<F, T, P, B> Session<F, T, P, B> {
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
        self.compositor.has_frame()
    }

    pub fn has_links(&self) -> bool {
        self.browsing.content_regions().next().is_some()
    }
}
