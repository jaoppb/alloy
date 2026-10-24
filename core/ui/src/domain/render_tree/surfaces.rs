//! How the viewport divides into the chrome strip and the content below it.

use graphics::AU_PER_PX;

use super::tree::UiRenderTree;
use crate::domain::error::UiError;

impl UiRenderTree {
    /// The surface size the chrome bar is rasterized into.
    ///
    /// # Errors
    ///
    /// Returns [`UiError::InvalidDimensions`] if the chrome height is zero.
    pub fn chrome_size(&self) -> Result<graphics::SurfaceSize, UiError> {
        graphics::SurfaceSize::new(self.viewport().width(), self.chrome_rows()?)
            .ok_or(UiError::InvalidDimensions)
    }

    /// The surface size available for laying out web content.
    ///
    /// # Errors
    ///
    /// Returns [`UiError::InvalidDimensions`] if the content height is zero.
    pub fn content_size(&self) -> Result<graphics::SurfaceSize, UiError> {
        let content_height = self
            .viewport()
            .height()
            .checked_sub(self.chrome_rows()?)
            .ok_or(UiError::InvalidDimensions)?;
        graphics::SurfaceSize::new(self.viewport().width(), content_height)
            .ok_or(UiError::InvalidDimensions)
    }

    /// The whole pixel rows the chrome bar covers. Content takes the rows
    /// left over, so the two surfaces always stack to the full viewport.
    fn chrome_rows(&self) -> Result<u32, UiError> {
        let whole_pixels = self
            .chrome_height()
            .to_au()
            .raw()
            .checked_div(AU_PER_PX)
            .ok_or(UiError::InvalidDimensions)?;
        u32::try_from(whole_pixels).map_err(|_| UiError::InvalidDimensions)
    }
}
