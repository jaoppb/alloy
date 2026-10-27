//! The layered compositor (`ADR-0019` addendum, #108): the chrome and the
//! content are rasterized into separate cached layers, so a change to one
//! never re-rasters the other.
//!
//! The compositor owns the raster backends and reaches them only through the
//! [`RenderBackend`] port; the composition root decides which backend that is.
//! It knows nothing of sessions, omniboxes' behavior or navigation — it is
//! handed what to draw and tells the caller whether a frame went out.

mod damage;
mod frame;
mod pixel;

use std::sync::Arc;

use browser::{Document, LinkTargets};
use graphics::{DisplayListBuilder, FontProvider, RenderBackend, SurfaceSize as LayerSize};
use ui::{OmniboxPainter, OmniboxState, OmniboxTheme};
use window::{Presenter, SurfaceSize};

pub use self::damage::{Damage, PendingDamage};
use self::frame::{ComposedFrame, Layer};
use crate::application::paint::DEFAULT_FONT;
use crate::application::pipeline::{render_dom_with_links, surface_length_to_au};
use crate::error::AlloyError;

/// What the content layer holds.
enum ContentLayer {
    /// No document has been laid out yet: nothing can be composed.
    Pending,
    /// The viewport leaves no rows below the chrome (#118); valid, and shown
    /// as the chrome alone.
    Empty,
    Raster(Layer),
}

/// Two cached layers and the last frame composed from them.
pub struct Compositor<B> {
    chrome_backend: B,
    content_backend: B,
    chrome: Option<Layer>,
    content: ContentLayer,
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
            chrome: None,
            content: ContentLayer::Pending,
            frame: None,
        }
    }

    /// Re-rasters the chrome layer from the omnibox as it is now. Leaves the
    /// content layer untouched.
    pub fn raster_chrome<F: FontProvider>(
        &mut self,
        omnibox: &OmniboxState,
        theme: &OmniboxTheme,
        size: LayerSize,
        fonts: &F,
    ) -> Result<(), AlloyError> {
        let mut builder = DisplayListBuilder::new();
        OmniboxPainter::new(fonts)
            .with_theme(*theme)
            .with_font(DEFAULT_FONT)
            .paint(
                omnibox,
                surface_length_to_au(size.width())?.to_px(),
                &mut builder,
            )?;
        let display_list = builder.build()?;

        self.chrome_backend.begin_frame(size)?;
        self.chrome_backend.submit(&display_list)?;
        self.chrome_backend.end_frame()?;
        self.chrome = Some(Layer::from_framebuffer(&self.chrome_backend.read_back()?));
        Ok(())
    }

    /// Lays out, paints and rasters `document` into the content layer, and
    /// returns the clickable links of that layout.
    pub fn raster_content<F: FontProvider + 'static>(
        &mut self,
        document: &Document,
        size: LayerSize,
        fonts: &Arc<F>,
    ) -> Result<LinkTargets, AlloyError> {
        let (framebuffer, links) = render_dom_with_links(
            document.tree(),
            document.sheets().clone(),
            document.images(),
            size,
            fonts,
            &mut self.content_backend,
        )?;
        self.content = ContentLayer::Raster(Layer::from_framebuffer(&framebuffer));
        Ok(links)
    }

    /// Marks the content layer as having no rows: the viewport is no taller
    /// than the chrome.
    pub fn clear_content(&mut self) {
        self.content = ContentLayer::Empty;
    }

    /// Composes the layers at `viewport` and presents the result. Returns
    /// whether a frame went out: it does not until both layers have been
    /// rastered at least once.
    pub fn present<R: Presenter>(
        &mut self,
        viewport: SurfaceSize,
        presenter: &mut R,
    ) -> Result<bool, AlloyError> {
        let Some(chrome) = self.chrome.as_ref() else {
            return Ok(false);
        };
        let content = match &self.content {
            ContentLayer::Pending => return Ok(false),
            ContentLayer::Empty => None,
            ContentLayer::Raster(layer) => Some(layer),
        };
        let frame = ComposedFrame::compose(chrome, content, viewport)?;
        frame.present(presenter)?;
        self.frame = Some(frame);
        Ok(true)
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

    #[cfg(test)]
    pub const fn has_frame(&self) -> bool {
        self.frame.is_some()
    }
}
