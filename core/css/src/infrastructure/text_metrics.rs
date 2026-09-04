//! [`MonospaceMetrics`] — a deterministic [`TextMeasurer`] placeholder.
//!
//! Every glyph advances a fixed `0.6 × font-size` and a line is `1.2 ×
//! font-size` tall — expressed as the exact integer fractions `3/5` and `6/5`
//! of the [`Au`] font size so the result is byte-identical on every platform.
//! B3 introduces a real font-backed measurer.

use graphics::Au;

use crate::application::ports::TextMeasurer;
use crate::domain::error::{CssError, CssStage};
use crate::domain::text::{ComputedText, TextMetrics, TextRun};

/// Numerator / denominator of the fixed glyph advance (`0.6`).
const ADVANCE_NUMERATOR: i32 = 3;
const ADVANCE_DENOMINATOR: i32 = 5;
/// Numerator / denominator of the fixed line height (`1.2`).
const LINE_HEIGHT_NUMERATOR: i32 = 6;
const LINE_HEIGHT_DENOMINATOR: i32 = 5;

/// A fixed-advance, single-line-height measurer.
#[derive(Clone, Copy, Debug, Default)]
pub struct MonospaceMetrics;

impl MonospaceMetrics {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl TextMeasurer for MonospaceMetrics {
    fn measure(&self, run: &TextRun, style: &ComputedText) -> Result<TextMetrics, CssError> {
        let font_size = style.font_size().raw();
        let advance = scale(font_size, ADVANCE_NUMERATOR, ADVANCE_DENOMINATOR)
            .ok_or_else(|| overflow("font size too large to measure"))?;
        let glyphs = i32::try_from(run.char_count())
            .map_err(|_overflow| overflow("text run too long to measure"))?;
        let width = advance
            .checked_mul(glyphs)
            .ok_or_else(|| overflow("text run too long to measure"))?;
        let height = scale(font_size, LINE_HEIGHT_NUMERATOR, LINE_HEIGHT_DENOMINATOR)
            .ok_or_else(|| overflow("font size too large to measure"))?;
        Ok(TextMetrics::new(Au::from_raw(width), Au::from_raw(height)))
    }
}

fn scale(value: i32, numerator: i32, denominator: i32) -> Option<i32> {
    value.checked_mul(numerator)?.checked_div(denominator)
}

fn overflow(detail: &'static str) -> CssError {
    CssError::unsupported(CssStage::Measure, detail)
}
