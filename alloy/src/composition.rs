//! What the composition root decides before the loop starts.

use window::{SurfaceSize, WindowAttributes, WindowTitle};

use crate::application::pipeline::RenderOptions;
use crate::error::AlloyError;

/// The window `alloy <url>` and the e2e golden test both open.
///
/// One shared definition so the caller and the loop's viewport-tracking agree
/// on the starting size.
///
/// # Errors
///
/// [`AlloyError::InvalidDimensions`] only if [`RenderOptions`]'s own defaults
/// were ever changed to zero — not reachable with the values in this crate.
pub fn initial_window_attributes() -> Result<WindowAttributes, AlloyError> {
    let size = SurfaceSize::new(RenderOptions::DEFAULT_WIDTH, RenderOptions::DEFAULT_HEIGHT)
        .ok_or(AlloyError::InvalidDimensions)?;
    Ok(WindowAttributes::new(WindowTitle::from("alloy"), size))
}
