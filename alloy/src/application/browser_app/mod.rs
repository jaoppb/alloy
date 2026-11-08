//! The state one browser session keeps between frames, and how it is entered
//! (`ADR-0019`).
//!
//! The document — and the rules for which fetch results may change it — live in
//! `core/browser`; the chrome lives in `core/ui`. This type holds both, hands
//! them to the [`EffectExecutor`] for each dispatch, and gives the loop the
//! handful of operations a pump cycle needs. Every field is private: `damage` →
//! [`BrowserApp::render`] → clean, and the context's link regions always
//! describing the content layer last rastered, are invariants only this type's
//! methods may move.

mod dispatch;
mod render;

#[cfg(test)]
mod click_tests;
#[cfg(test)]
mod coalescing_tests;
#[cfg(test)]
mod damage_tests;
#[cfg(test)]
mod keyboard_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

use browser::{BrowsingContext, FetchResult, MarkupDiscoverer};
use graphics::{FontProvider, RenderBackend};
use network::Url;
use platform::HostPlatform;
use ui::{Keymap, Theme, UiController, UiRenderTree};
use window::{InputText, KeyEvent, PhysicalPosition, Presenter, SurfaceSize};

use self::render::ui_tree_for;
use crate::application::browser_services::BrowserServices;
use crate::application::compositor::{Compositor, Damage, PendingDamage};
use crate::application::effects::EffectExecutor;
use crate::application::event_loop;
use crate::application::keymap::resolve_keymap;
use crate::application::loop_stats::LoopStats;
use crate::application::ports::{FetchExecutor, IdlePause, KeymapSource};
use crate::error::AlloyError;

/// One browser session: browsing context, chrome, pending damage and the
/// compositor that turns them into frames.
pub struct BrowserApp<F, X, W, B> {
    services: BrowserServices<F, X, W>,
    browsing: BrowsingContext<MarkupDiscoverer>,
    pointer_position: Option<PhysicalPosition>,
    damage: PendingDamage,
    viewport: SurfaceSize,
    compositor: Compositor<B>,
    stats: LoopStats,
    /// Focus, key routing and the omnibox.
    ui: UiController,
    /// The one source of chrome geometry: the chrome is painted with it and
    /// `ui_tree` is laid out from it, so clicks land where pixels are drawn.
    theme: Theme,
    ui_tree: UiRenderTree,
}

impl<F, X, W, B> BrowserApp<F, X, W, B>
where
    F: FontProvider + 'static,
    X: FetchExecutor,
    W: IdlePause,
    B: RenderBackend,
{
    /// A session at `viewport` whose router resolves keys with `keymap`.
    pub fn new(
        viewport: SurfaceSize,
        services: BrowserServices<F, X, W>,
        compositor: Compositor<B>,
        keymap: Keymap,
    ) -> Self {
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
            stats: LoopStats::default(),
            ui: UiController::new(keymap, HostPlatform::new().os()),
            theme,
            ui_tree,
        }
    }

    /// Navigates to `url`, then pumps until the window closes.
    ///
    /// `system` and `presenter` must already have a live window; see
    /// [`crate::composition::initial_window_attributes`].
    pub fn run<S, R>(
        &mut self,
        url: &Url,
        system: &mut S,
        presenter: &mut R,
    ) -> Result<LoopStats, AlloyError>
    where
        S: window::WindowSystem,
        R: Presenter,
    {
        self.run_until(url, system, presenter, |_| false)
    }

    /// [`Self::run`], but returns as soon as `should_stop` answers `true` for
    /// the accumulated [`LoopStats`] — a test waiting for specific background
    /// work without racing the fetch threads against a scripted close.
    pub fn run_until<S, R>(
        &mut self,
        url: &Url,
        system: &mut S,
        presenter: &mut R,
        should_stop: impl FnMut(&LoopStats) -> bool,
    ) -> Result<LoopStats, AlloyError>
    where
        S: window::WindowSystem,
        R: Presenter,
    {
        self.navigate(url.clone());
        event_loop::run_loop(system, presenter, self, should_stop)
    }

    /// [`Self::run_until`], stopping as soon as the first frame has been laid
    /// out and presented.
    pub fn run_until_first_frame<S, R>(
        &mut self,
        url: &Url,
        system: &mut S,
        presenter: &mut R,
    ) -> Result<LoopStats, AlloyError>
    where
        S: window::WindowSystem,
        R: Presenter,
    {
        self.run_until(url, system, presenter, |stats| stats.relayouts() >= 1)
    }

    /// What the loop has done so far.
    pub const fn stats(&self) -> LoopStats {
        self.stats.observing(self.browsing.stats())
    }

    /// Replaces the key bindings with what `source` supplies now (init or reload).
    pub fn reload_keymap(&mut self, source: &impl KeymapSource) {
        self.ui.set_keymap(resolve_keymap(source));
    }

    /// Starts navigating to `url`.
    pub fn navigate(&mut self, url: Url) {
        self.executor().navigate(url);
    }

    /// Routes a key event through the chrome and executes what it asks for.
    pub fn handle_key(&mut self, key: &KeyEvent) {
        let effects = self.ui.handle_key(key);
        self.executor().execute_ui(effects);
    }

    /// Routes an input-method commit through the chrome.
    pub fn handle_text_committed(&mut self, text: InputText) {
        let effects = self.ui.handle_text_committed(text);
        self.executor().execute_ui(effects);
    }

    /// Dispatches a primary-button press at `position` through the chrome.
    pub fn handle_click(&mut self, position: PhysicalPosition) {
        let effects = self.ui.handle_click(&self.ui_tree, position);
        self.executor().execute_ui(effects);
    }

    /// The next finished fetch already waiting, if any.
    pub fn next_ready_fetch(&self) -> Option<FetchResult> {
        self.services.next_ready_fetch()
    }

    /// Applies one finished fetch.
    pub fn complete_fetch(&mut self, result: FetchResult) {
        self.executor().complete(result);
    }

    /// Where the pointer was last seen — `None` until it first enters.
    pub const fn pointer_position(&self) -> Option<PhysicalPosition> {
        self.pointer_position
    }

    pub const fn track_pointer(&mut self, position: Option<PhysicalPosition>) {
        self.pointer_position = position;
    }

    /// Adopts the new viewport; the next [`Self::render`] sizes both layers to it.
    pub fn resize(&mut self, viewport: SurfaceSize) {
        self.viewport = viewport;
        self.ui_tree = ui_tree_for(viewport, &self.theme, self.browsing.content_regions());
        self.damage.record(Damage::Viewport);
    }

    /// Waits briefly after a pump cycle that found nothing to do.
    pub fn pause_idle(&self) {
        self.services.pause_idle();
    }

    /// Presents the last composed frame again, with no raster work and without
    /// touching the relayout count.
    pub fn repaint<R: Presenter>(&self, presenter: &mut R) -> Result<(), AlloyError> {
        self.compositor.repaint(presenter)
    }

    const fn executor(&mut self) -> EffectExecutor<'_, X> {
        EffectExecutor::new(
            &mut self.browsing,
            &mut self.ui,
            &mut self.damage,
            self.services.fetch_executor(),
        )
    }
}
