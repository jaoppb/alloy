//! [`SyntheticFontProvider`] — the deterministic [`FontProvider`] every golden
//! and conformance test uses.
//!
//! No filesystem, no real glyph shapes: every glyph a registered font can
//! produce is the same fixed-size filled block, sized as an exact integer
//! fraction of the registered size — the same technique
//! [`crate::MonospaceMetrics`]'s `core/css` counterpart uses, so both halves of
//! a golden agree on what "deterministic" means. Bit-identical on every OS,
//! which is the entire point (`ADR-0016`).

use std::collections::BTreeMap;

use crate::application::FontProvider;
use crate::domain::error::GraphicsError;
use crate::domain::font::{FaceMetrics, FontId, GlyphBitmap, GlyphId};
use crate::domain::geometry::Point;
use crate::domain::unit::{AU_PER_PX, Au};

/// Numerator / denominator of the fixed glyph block width (`0.6`) — matches
/// `core/css`'s `MonospaceMetrics` advance fraction.
const BLOCK_WIDTH_NUMERATOR: i32 = 3;
const BLOCK_WIDTH_DENOMINATOR: i32 = 5;
/// Numerator / denominator of the fixed glyph block height (`0.7`).
const BLOCK_HEIGHT_NUMERATOR: i32 = 7;
const BLOCK_HEIGHT_DENOMINATOR: i32 = 10;
/// Numerator / denominator of the fixed ascent (`0.8`).
const ASCENT_NUMERATOR: i32 = 4;
const ASCENT_DENOMINATOR: i32 = 5;
/// Numerator / denominator of the fixed descent (`0.2`).
const DESCENT_NUMERATOR: i32 = 1;
const DESCENT_DENOMINATOR: i32 = 5;

/// A deterministic, filesystem-free [`FontProvider`].
#[derive(Clone, Debug, Default)]
pub struct SyntheticFontProvider {
    sizes: BTreeMap<FontId, Au>,
}

impl SyntheticFontProvider {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `id` to render at `size`.
    #[must_use]
    pub fn with_size(mut self, id: FontId, size: Au) -> Self {
        self.sizes.insert(id, size);
        self
    }

    fn size_of(&self, font: FontId) -> Result<Au, GraphicsError> {
        self.sizes
            .get(&font)
            .copied()
            .ok_or(GraphicsError::FontUnavailable { font })
    }
}

impl FontProvider for SyntheticFontProvider {
    fn rasterize(&self, font: FontId, glyph: GlyphId) -> Result<GlyphBitmap, GraphicsError> {
        let size = self.size_of(font)?;
        // `.notdef` is the one glyph index every face reserves, and this
        // provider treats it as "nothing to draw" — the synthetic stand-in for
        // whitespace, since a bare `GlyphId` carries no character information
        // for this provider to test against.
        if glyph == GlyphId::NOTDEF {
            return Ok(GlyphBitmap::empty());
        }
        let width = fraction_px(size, BLOCK_WIDTH_NUMERATOR, BLOCK_WIDTH_DENOMINATOR);
        let height = fraction_px(size, BLOCK_HEIGHT_NUMERATOR, BLOCK_HEIGHT_DENOMINATOR);
        if width == 0 || height == 0 {
            return Ok(GlyphBitmap::empty());
        }
        let cell_count = usize::try_from(width)
            .ok()
            .zip(usize::try_from(height).ok())
            .and_then(|(width, height)| width.checked_mul(height))
            .unwrap_or(0);
        let coverage = vec![u8::MAX; cell_count];
        let ascent = fraction_au(size, ASCENT_NUMERATOR, ASCENT_DENOMINATOR);
        let bearing = Point::new(Au::ZERO, Au::ZERO.saturating_sub(ascent));
        GlyphBitmap::new(width, height, coverage, bearing)
            .ok_or(GraphicsError::FontUnavailable { font })
    }

    fn metrics(&self, font: FontId) -> Result<FaceMetrics, GraphicsError> {
        let size = self.size_of(font)?;
        let ascent = fraction_au(size, ASCENT_NUMERATOR, ASCENT_DENOMINATOR);
        let descent = fraction_au(size, DESCENT_NUMERATOR, DESCENT_DENOMINATOR);
        Ok(FaceMetrics::new(ascent, descent, Au::ZERO))
    }

    fn glyph_for_char(&self, font: FontId, character: char) -> Result<GlyphId, GraphicsError> {
        self.size_of(font)?;
        if character.is_whitespace() {
            return Ok(GlyphId::NOTDEF);
        }
        Ok(GlyphId::new(1))
    }

    /// Every glyph — including the whitespace stand-in [`GlyphId::NOTDEF`] —
    /// advances by the same fixed block width. Monospaced by construction,
    /// which is exactly what makes this provider deterministic.
    fn advance(&self, font: FontId, _glyph: GlyphId) -> Result<Au, GraphicsError> {
        let size = self.size_of(font)?;
        Ok(fraction_au(
            size,
            BLOCK_WIDTH_NUMERATOR,
            BLOCK_WIDTH_DENOMINATOR,
        ))
    }
}

fn fraction_au(size: Au, numerator: i32, denominator: i32) -> Au {
    let scaled = size
        .raw()
        .checked_mul(numerator)
        .and_then(|value| value.checked_div(denominator))
        .unwrap_or(0);
    Au::from_raw(scaled)
}

fn fraction_px(size: Au, numerator: i32, denominator: i32) -> u32 {
    let au = fraction_au(size, numerator, denominator);
    let whole_pixels = au.raw().checked_div(AU_PER_PX).unwrap_or(0);
    u32::try_from(whole_pixels).unwrap_or(0)
}
