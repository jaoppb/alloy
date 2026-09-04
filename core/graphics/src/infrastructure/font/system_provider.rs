//! [`SystemFontProvider`] — resolves a CSS generic font family to a real,
//! installed font, via [`FontCatalog`] and [`TtfParserProvider`].
//!
//! Best-effort by design: a container or a minimal CI image may have none of
//! the candidate paths installed, and that is a legitimate
//! [`GraphicsError::FontUnavailable`], never a panic. Nothing in the golden or
//! conformance suite depends on this adapter finding a font — those always use
//! [`crate::infrastructure::font::SyntheticFontProvider`].

use crate::application::FontProvider;
use crate::domain::error::GraphicsError;
use crate::domain::font::{FaceMetrics, FontId, GlyphBitmap, GlyphId};
use crate::domain::unit::Au;
use crate::infrastructure::font::catalog::{FontCatalog, GenericFamily};
use crate::infrastructure::font::ttf_provider::TtfParserProvider;

/// A [`FontProvider`] over one real, resolved system font.
#[derive(Debug)]
pub struct SystemFontProvider {
    inner: TtfParserProvider,
}

impl SystemFontProvider {
    /// Resolves `family` to the first candidate path that exists and parses,
    /// registers it as `id` at `size`.
    ///
    /// # Errors
    ///
    /// [`GraphicsError::FontUnavailable`] when none of [`FontCatalog`]'s
    /// candidates for `family` exist and parse on this machine.
    pub fn resolve(family: GenericFamily, id: FontId, size: Au) -> Result<Self, GraphicsError> {
        for path in FontCatalog::candidate_paths(family) {
            let Ok(data) = std::fs::read(path) else {
                continue;
            };
            if let Ok(provider) = TtfParserProvider::new().with_face(id, data, size) {
                return Ok(Self { inner: provider });
            }
        }
        Err(GraphicsError::FontUnavailable { font: id })
    }
}

impl FontProvider for SystemFontProvider {
    fn rasterize(&self, font: FontId, glyph: GlyphId) -> Result<GlyphBitmap, GraphicsError> {
        self.inner.rasterize(font, glyph)
    }

    fn metrics(&self, font: FontId) -> Result<FaceMetrics, GraphicsError> {
        self.inner.metrics(font)
    }

    fn glyph_for_char(&self, font: FontId, character: char) -> Result<GlyphId, GraphicsError> {
        self.inner.glyph_for_char(font, character)
    }

    fn advance(&self, font: FontId, glyph: GlyphId) -> Result<Au, GraphicsError> {
        self.inner.advance(font, glyph)
    }
}
