//! Turning pending damage into a presented frame.

use browser::LinkTargets;
use graphics::{FontProvider, RenderBackend};
use region::ContentRegion;
use ui::{Theme, UiRenderTree};
use window::{Presenter, SurfaceSize};

use super::BrowserApp;
use crate::application::compositor::{
    ChromeScene, ContentScene, Layout, PendingDamage, Presented, Rendered, Scene,
};
use crate::application::ports::{FetchExecutor, IdlePause};
use crate::error::AlloyError;

impl<F, X, W, B> BrowserApp<F, X, W, B>
where
    F: FontProvider + 'static,
    X: FetchExecutor,
    W: IdlePause,
    B: RenderBackend,
{
    /// Rasters whatever the accumulated damage invalidated, composes the
    /// layers and presents once.
    ///
    /// Chrome damage re-rasters the chrome strip only. Content damage is the
    /// relayout — the only path that counts a relayout, and only once a frame
    /// laid out from a document was actually presented (the I4 coalescing proof
    /// and `run_until_first_frame` both rest on that). With no document yet
    /// there is nothing to lay out, and that damage is dropped. A render that
    /// fails puts its damage back, so nothing it consumed is lost.
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
                self.ui.omnibox().state(),
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
            self.stats.record_relayout();
        }
    }
}

/// The UI tree for `viewport`, or the inert fallback when the viewport cannot
/// hold the chrome; the compositor then shows the chrome alone (#118).
pub(super) fn ui_tree_for(
    viewport: SurfaceSize,
    theme: &Theme,
    regions: impl IntoIterator<Item = ContentRegion>,
) -> UiRenderTree {
    UiRenderTree::build(viewport, theme.omnibox(), regions)
        .unwrap_or_else(|_| UiRenderTree::fallback(viewport, theme.omnibox()))
}
