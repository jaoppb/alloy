//! The layered compositor (`ADR-0019` addendum, #108): the chrome and the
//! content are rasterized into separate cached layers, so a change to one
//! never re-rasters the other.
//!
//! The compositor owns the raster backends and reaches them only through the
//! [`RenderBackend`] port; the composition root decides which backend that is.
//! It is handed a [`Scene`] and the accumulated [`PendingDamage`], decides
//! which layers to re-raster, and tells the caller what it laid out and
//! whether a frame went out. It knows nothing of sessions or navigation.

mod damage;
mod frame;
mod pixel;
mod scene;

use browser::LinkTargets;
use graphics::{DisplayListBuilder, FontProvider, RenderBackend};
use ui::OmniboxPainter;
use window::Presenter;

pub use self::damage::{Damage, PendingDamage};
use self::frame::{ComposedFrame, Layer};
pub use self::scene::{ChromeScene, ContentScene, Scene};
use crate::application::paint::DEFAULT_FONT;
use crate::application::pipeline::{render_dom_with_links, surface_length_to_au};
use crate::error::AlloyError;

/// What one layer holds.
enum LayerSlot {
    /// Never rastered: nothing can be composed yet.
    Pending,
    /// The viewport leaves no rows for this layer (#118); valid, and shown as
    /// nothing.
    Empty,
    Raster(Layer),
}

impl LayerSlot {
    const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    const fn layer(&self) -> Option<&Layer> {
        match self {
            Self::Raster(layer) => Some(layer),
            Self::Pending | Self::Empty => None,
        }
    }
}

/// Whether a render put a frame on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presented {
    Shown,
    /// A layer has never been rastered, so there is nothing to compose.
    NotYet,
    /// No damage was taken, so nothing was composed or presented. Keeps an
    /// idle session idle: the caller must not re-arm a redraw for this.
    Unchanged,
}

impl Presented {
    #[must_use]
    pub const fn is_shown(self) -> bool {
        matches!(self, Self::Shown)
    }
}

/// What a render did to the content layer.
pub enum Layout {
    /// The content layer was not touched: no content damage, or no document
    /// to lay out (that damage is dropped).
    Untouched,
    /// The viewport leaves no rows below the chrome.
    NoRows,
    /// The document was laid out; these are its clickable links.
    Done(LinkTargets),
}

/// The outcome of [`Compositor::render`].
pub struct Rendered {
    presented: Presented,
    layout: Layout,
}

impl Rendered {
    #[must_use]
    pub const fn presented(&self) -> Presented {
        self.presented
    }

    /// Hands over what happened to the content layer, so the caller can commit
    /// the links only after the frame that shows them is out.
    #[must_use]
    pub fn into_layout(self) -> Layout {
        self.layout
    }
}

/// Two cached layers and the last frame composed from them.
pub struct Compositor<B> {
    chrome_backend: B,
    content_backend: B,
    chrome: LayerSlot,
    content: LayerSlot,
    frame: Option<ComposedFrame>,
}

impl<B: RenderBackend> Compositor<B> {
    /// A compositor rasterizing the chrome with `chrome_backend` and the page
    /// with `content_backend`. Two instances because the content backend's
    /// images follow the document while the chrome's never do.
    #[must_use]
    pub const fn new(chrome_backend: B, content_backend: B) -> Self {
        Self {
            chrome_backend,
            content_backend,
            chrome: LayerSlot::Pending,
            content: LayerSlot::Pending,
            frame: None,
        }
    }

    /// Re-rasters the layers `damage` invalidated from `scene`, composes them
    /// and presents once.
    ///
    /// Without damage nothing is composed or presented ([`Presented::Unchanged`]);
    /// re-showing the last frame is [`Self::repaint`]'s job. Nothing is shown
    /// until both layers have been rastered at least once. Content damage with no document in the scene is dropped.
    pub fn render<F: FontProvider + 'static, R: Presenter>(
        &mut self,
        damage: PendingDamage,
        scene: &Scene<'_, F>,
        presenter: &mut R,
    ) -> Result<Rendered, AlloyError> {
        if damage.is_empty() {
            return Ok(Rendered {
                presented: Presented::Unchanged,
                layout: Layout::Untouched,
            });
        }
        if damage.needs_chrome() {
            self.raster_chrome(scene)?;
        }
        let layout = if damage.needs_content() {
            self.raster_content(scene)?
        } else {
            Layout::Untouched
        };
        let shown = self.present(scene, presenter)?;
        Ok(Rendered {
            presented: shown,
            layout,
        })
    }

    /// Presents the last composed frame again, with no raster work. A no-op
    /// before the first frame. Serves `RedrawRequested`: a compositor
    /// expose/occlusion, or the redraw winit re-arms once a Wayland surface
    /// that dropped the first present is finally configured.
    pub fn repaint<R: Presenter>(&self, presenter: &mut R) -> Result<(), AlloyError> {
        let Some(frame) = self.frame.as_ref() else {
            return Ok(());
        };
        frame.present(presenter)
    }

    fn raster_chrome<F: FontProvider + 'static>(
        &mut self,
        scene: &Scene<'_, F>,
    ) -> Result<(), AlloyError> {
        let chrome = scene.chrome();
        let mut builder = DisplayListBuilder::new();
        OmniboxPainter::new(scene.fonts().as_ref())
            .with_theme(*chrome.theme())
            .with_font(DEFAULT_FONT)
            .paint(
                chrome.omnibox(),
                surface_length_to_au(chrome.size().width())?.to_px(),
                &mut builder,
            )?;
        let display_list = builder.build()?;

        self.chrome_backend.begin_frame(chrome.size())?;
        self.chrome_backend.submit(&display_list)?;
        self.chrome_backend.end_frame()?;
        let framebuffer = self.chrome_backend.read_back()?;
        self.chrome = LayerSlot::Raster(Layer::from_framebuffer(&framebuffer));
        Ok(())
    }

    fn raster_content<F: FontProvider + 'static>(
        &mut self,
        scene: &Scene<'_, F>,
    ) -> Result<Layout, AlloyError> {
        let content = scene.content();
        let Some(document) = content.document() else {
            return Ok(Layout::Untouched);
        };
        let Some(size) = content.size() else {
            self.content = LayerSlot::Empty;
            return Ok(Layout::NoRows);
        };
        let (framebuffer, links) = render_dom_with_links(
            document.tree(),
            document.sheets().clone(),
            document.images(),
            size,
            scene.fonts(),
            &mut self.content_backend,
        )?;
        self.content = LayerSlot::Raster(Layer::from_framebuffer(&framebuffer));
        Ok(Layout::Done(links))
    }

    fn present<F, R: Presenter>(
        &mut self,
        scene: &Scene<'_, F>,
        presenter: &mut R,
    ) -> Result<Presented, AlloyError> {
        if self.chrome.is_pending() || self.content.is_pending() {
            return Ok(Presented::NotYet);
        }
        let Some(chrome) = self.chrome.layer() else {
            return Ok(Presented::NotYet);
        };
        let frame = ComposedFrame::compose(chrome, self.content.layer(), scene.viewport())?;
        frame.present(presenter)?;
        self.frame = Some(frame);
        Ok(Presented::Shown)
    }
}

impl<B> Compositor<B> {
    #[cfg(test)]
    pub const fn has_frame(&self) -> bool {
        self.frame.is_some()
    }
}
