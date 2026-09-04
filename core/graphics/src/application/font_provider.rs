//! [`FontProvider`] — the replaceable font-rasterization port (v0.5 B3).
//!
//! `DrawText` carries a [`FontId`] and an already-shaped, already-positioned
//! [`GlyphRun`] (`PRD-005:62-63` — shaping happens before the display list, not
//! in the backend). What the backend still needs, and does not itself know how
//! to produce, is *pixels* for each glyph: a `FontProvider` turns a `(font,
//! glyph)` pair into a [`GlyphBitmap`] the backend blits, and reports a face's
//! vertical metrics for line-height math upstream in `core/css`.
//!
//! Object-safe and speaks only this crate's own types — no `ttf-parser` type
//! crosses this seam, the same discipline [`RenderBackend`](super::RenderBackend)
//! already holds. Three adapters exist: [`TtfParserProvider`] (real font
//! files), [`SystemFontProvider`] (resolves `sans-serif` / `serif` /
//! `monospace` to installed fonts, backed by [`TtfParserProvider`]), and
//! [`SyntheticFontProvider`] (deterministic, no filesystem — the one every
//! golden and conformance test uses).
//!
//! [`TtfParserProvider`]: crate::infrastructure::font::TtfParserProvider
//! [`SystemFontProvider`]: crate::infrastructure::font::SystemFontProvider
//! [`SyntheticFontProvider`]: crate::infrastructure::font::SyntheticFontProvider

use crate::domain::error::GraphicsError;
use crate::domain::font::{FaceMetrics, FontId, GlyphBitmap, GlyphId};
use crate::domain::unit::Au;

/// Resolves a registered [`FontId`] to rasterized glyphs, face metrics, and —
/// for `core/css`'s real `TextMeasurer` adapter (v0.5 B3) — glyph lookup and
/// advance width.
pub trait FontProvider: Send + Sync {
    /// The coverage mask for `glyph` in `font`, at the size `font` was
    /// registered at.
    ///
    /// An unmapped glyph (e.g. whitespace) is [`GlyphBitmap::empty`] — `Ok`,
    /// not an error. Only an unregistered `font` is
    /// [`GraphicsError::FontUnavailable`].
    fn rasterize(&self, font: FontId, glyph: GlyphId) -> Result<GlyphBitmap, GraphicsError>;

    /// The vertical metrics of `font` at its registered size.
    fn metrics(&self, font: FontId) -> Result<FaceMetrics, GraphicsError>;

    /// The glyph `font`'s `cmap` maps `character` to.
    ///
    /// A character with no mapping is [`GlyphId::NOTDEF`] — `Ok`, not an
    /// error, the same convention every outline font format uses. Only an
    /// unregistered `font` is [`GraphicsError::FontUnavailable`].
    fn glyph_for_char(&self, font: FontId, character: char) -> Result<GlyphId, GraphicsError>;

    /// How far the pen advances after painting `glyph`, at `font`'s
    /// registered size.
    fn advance(&self, font: FontId, glyph: GlyphId) -> Result<Au, GraphicsError>;
}
