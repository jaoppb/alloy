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
    /// Composes the chrome top-bar framebuffer and the web content framebuffer
    /// into a single presented frame matching `viewport` dimensions.
    ///
    /// # Errors
    ///
    /// Returns [`AlloyError::InvalidDimensions`] if the total combined pixel count
    /// does not match `viewport.width() * viewport.height()`.
    pub fn compose(
        chrome: &Framebuffer,
        content: &Framebuffer,
        viewport: SurfaceSize,
    ) -> Result<Self, AlloyError> {
        let chrome_pixels = frame_pixels(chrome);
        let content_pixels = frame_pixels(content);
        let mut pixels =
            Vec::with_capacity(chrome_pixels.len().saturating_add(content_pixels.len()));
        pixels.extend(chrome_pixels);
        pixels.extend(content_pixels);

        let width_usize =
            usize::try_from(viewport.width()).map_err(|_| AlloyError::InvalidDimensions)?;
        let height_usize =
            usize::try_from(viewport.height()).map_err(|_| AlloyError::InvalidDimensions)?;
        let expected_len = width_usize.saturating_mul(height_usize);
        if pixels.len() != expected_len {
            return Err(AlloyError::InvalidDimensions);
        }

        Ok(Self {
            width: viewport.width(),
            height: viewport.height(),
            pixels,
        })
    }

    pub fn present<R: Presenter>(&self, presenter: &mut R) -> Result<(), AlloyError> {
        let view = FrameView::new(self.width, self.height, &self.pixels)
            .ok_or(AlloyError::InvalidDimensions)?;
        presenter.present(view)?;
        Ok(())
    }
}
