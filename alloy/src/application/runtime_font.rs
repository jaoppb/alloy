//! The font provider the running browser uses.

use graphics::{
    Au, FaceMetrics, FontId, FontProvider, GenericFamily, GlyphBitmap, GlyphId, GraphicsError,
    SyntheticFontProvider, SystemFontProvider,
};

use crate::application::paint::DEFAULT_FONT;
use crate::application::pipeline::DEFAULT_FONT_SIZE;

/// A system sans-serif face when the host has one, the deterministic
/// synthetic font otherwise.
///
/// A closed enum instead of a `dyn FontProvider`: the fallback choice is made
/// once at startup, and everything downstream stays statically dispatched.
pub enum RuntimeFontProvider {
    /// A real face read from the host's font directories.
    System(SystemFontProvider),
    /// The deterministic block-glyph fallback.
    Synthetic(SyntheticFontProvider),
}

impl RuntimeFontProvider {
    /// Resolves a system sans-serif font, falling back to
    /// [`SyntheticFontProvider`] (with a warning) if none exists or parses.
    #[must_use]
    pub fn resolve() -> Self {
        match SystemFontProvider::resolve(GenericFamily::SansSerif, DEFAULT_FONT, DEFAULT_FONT_SIZE)
        {
            Ok(system) => Self::System(system),
            Err(error) => {
                tracing::warn!(
                    %error,
                    "could not resolve system sans-serif font; falling back to synthetic font provider"
                );
                Self::Synthetic(
                    SyntheticFontProvider::new().with_size(DEFAULT_FONT, DEFAULT_FONT_SIZE),
                )
            }
        }
    }
}

impl FontProvider for RuntimeFontProvider {
    fn rasterize(&self, font: FontId, glyph: GlyphId) -> Result<GlyphBitmap, GraphicsError> {
        match self {
            Self::System(provider) => provider.rasterize(font, glyph),
            Self::Synthetic(provider) => provider.rasterize(font, glyph),
        }
    }

    fn metrics(&self, font: FontId) -> Result<FaceMetrics, GraphicsError> {
        match self {
            Self::System(provider) => provider.metrics(font),
            Self::Synthetic(provider) => provider.metrics(font),
        }
    }

    fn glyph_for_char(&self, font: FontId, character: char) -> Result<GlyphId, GraphicsError> {
        match self {
            Self::System(provider) => provider.glyph_for_char(font, character),
            Self::Synthetic(provider) => provider.glyph_for_char(font, character),
        }
    }

    fn advance(&self, font: FontId, glyph: GlyphId) -> Result<Au, GraphicsError> {
        match self {
            Self::System(provider) => provider.advance(font, glyph),
            Self::Synthetic(provider) => provider.advance(font, glyph),
        }
    }
}
