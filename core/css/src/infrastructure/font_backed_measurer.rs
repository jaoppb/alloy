//! [`FontBackedMeasurer`] — a real, font-backed [`TextMeasurer`] (v0.5 B3).
//!
//! Delegates every measurement to a `graphics::FontProvider`: glyph lookup,
//! per-glyph advance, and vertical face metrics. [`MonospaceMetrics`] stays
//! the default this crate's own tests use — a real font is only as
//! deterministic as the bytes backing it, and `SyntheticFontProvider` already
//! covers "deterministic and real-provider-shaped" for anyone who wants that
//! without installed fonts.
//!
//! [`MonospaceMetrics`]: crate::infrastructure::text_metrics::MonospaceMetrics

use std::sync::Arc;

use graphics::{Au, FontId, FontProvider};

use crate::application::ports::TextMeasurer;
use crate::domain::error::{CssError, CssStage};
use crate::domain::text::{ComputedText, TextMetrics, TextRun};

/// A [`TextMeasurer`] backed by a real `graphics::FontProvider`.
///
/// Holds one `font`: the caller (B4's inline layout) picks which registered
/// face/size this measurer answers for, matching how `DrawText` also names
/// one `FontId` per command.
pub struct FontBackedMeasurer {
    provider: Arc<dyn FontProvider>,
    font: FontId,
}

impl FontBackedMeasurer {
    #[must_use]
    pub const fn new(provider: Arc<dyn FontProvider>, font: FontId) -> Self {
        Self { provider, font }
    }
}

impl TextMeasurer for FontBackedMeasurer {
    fn measure(&self, run: &TextRun, style: &ComputedText) -> Result<TextMetrics, CssError> {
        let _ = style; // the registered FontId already fixes the size this measurer answers for
        let width = self.run_width(run)?;
        let metrics = self
            .provider
            .metrics(self.font)
            .map_err(|error| font_error(&error))?;
        Ok(TextMetrics::new(width, metrics.line_height()))
    }
}

impl FontBackedMeasurer {
    fn run_width(&self, run: &TextRun) -> Result<Au, CssError> {
        let mut width = Au::ZERO;
        for character in run.as_str().chars() {
            width = width.saturating_add(self.advance_for(character)?);
        }
        Ok(width)
    }

    fn advance_for(&self, character: char) -> Result<Au, CssError> {
        let glyph = self
            .provider
            .glyph_for_char(self.font, character)
            .map_err(|error| font_error(&error))?;
        self.provider
            .advance(self.font, glyph)
            .map_err(|error| font_error(&error))
    }
}

fn font_error(error: &graphics::GraphicsError) -> CssError {
    CssError::unsupported(CssStage::Measure, error.to_string())
}
