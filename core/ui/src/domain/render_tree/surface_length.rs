//! A surface edge as app units.

use graphics::Au;

use crate::domain::error::UiError;

/// A surface edge in whole pixels as app units, or
/// [`UiError::InvalidDimensions`] when it overflows the `Au` envelope.
///
/// `alloy`'s pipeline keeps its own copy because it reports `AlloyError`;
/// a shared `graphics` conversion would serve both (follow-up, not part of
/// this move).
pub(super) fn surface_length_to_au(pixels: u32) -> Result<Au, UiError> {
    let pixels = i32::try_from(pixels).map_err(|_| UiError::InvalidDimensions)?;
    Au::from_whole_px(pixels).ok_or(UiError::InvalidDimensions)
}
