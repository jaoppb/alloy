//! The pixels of one layer, and the frame composed from the layers.

use graphics::Framebuffer;
use window::{FrameView, Presenter, SurfaceSize};

use super::pixel::frame_pixels;
use crate::error::AlloyError;

/// One cached layer in the presenter's pixel format.
///
/// Held between damages so a change to the *other* layer recomposes without
/// re-rasterizing this one (`ADR-0019` addendum, #108).
pub struct Layer {
    width: u32,
    pixels: Vec<u32>,
}

impl Layer {
    pub fn from_framebuffer(framebuffer: &Framebuffer) -> Self {
        Self {
            width: framebuffer.width(),
            pixels: frame_pixels(framebuffer),
        }
    }
}

/// The pixels of the last frame the layers composed to, kept so a
/// `RedrawRequested` can re-blit them without re-rasterizing anything. This is
/// the "repaint is cheap, relayout is not" split the event loop rests on.
pub struct ComposedFrame {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
}

impl ComposedFrame {
    /// Stacks `chrome` over `content` into a frame of `viewport` size.
    ///
    /// A viewport shorter than the chrome shows the top of the chrome and no
    /// content; `content` is `None` exactly when it has no rows (#118).
    ///
    /// # Errors
    ///
    /// [`AlloyError::InvalidDimensions`] if the layers do not stack to exactly
    /// `viewport.width() * viewport.height()` pixels.
    pub fn compose(
        chrome: &Layer,
        content: Option<&Layer>,
        viewport: SurfaceSize,
    ) -> Result<Self, AlloyError> {
        let width_usize =
            usize::try_from(viewport.width()).map_err(|_| AlloyError::InvalidDimensions)?;
        let height_usize =
            usize::try_from(viewport.height()).map_err(|_| AlloyError::InvalidDimensions)?;
        let expected_len = width_usize.saturating_mul(height_usize);
        let visible_chrome = chrome.pixels.len().min(expected_len);

        let mut pixels = Vec::with_capacity(expected_len);
        pixels.extend_from_slice(chrome.pixels.get(..visible_chrome).unwrap_or_default());
        if let Some(content) = content {
            pixels.extend_from_slice(&content.pixels);
        }

        let widths_agree = chrome.width == viewport.width()
            && content.is_none_or(|layer| layer.width == viewport.width());
        if pixels.len() != expected_len || !widths_agree {
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
