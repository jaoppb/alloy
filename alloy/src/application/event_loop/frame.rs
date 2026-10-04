//! The last presented frame, kept for a cheap re-blit.

use graphics::Framebuffer;
use window::{FrameView, Presenter, SurfaceSize};

use super::pixel::frame_pixels;
use crate::error::AlloyError;

/// The pixels of the last frame a relayout produced, kept so a
/// `RedrawRequested` can re-blit them without re-running the whole
/// `render_dom_with_links` pipeline (cascade → layout → paint → raster →
/// readback). This is the "repaint is cheap, relayout is not" split the event
/// loop rests on.
pub struct CachedFrame {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
}

impl CachedFrame {
    /// Converts a framebuffer rendered at `viewport` into the presenter's
    /// premultiplied pixel format.
    pub fn capture(framebuffer: &Framebuffer, viewport: SurfaceSize) -> Self {
        Self {
            width: viewport.width(),
            height: viewport.height(),
            pixels: frame_pixels(framebuffer),
        }
    }

    pub fn present<R: Presenter>(&self, presenter: &mut R) -> Result<(), AlloyError> {
        let view = FrameView::new(self.width, self.height, &self.pixels)
            .ok_or(AlloyError::InvalidDimensions)?;
        presenter.present(view)?;
        Ok(())
    }
}
