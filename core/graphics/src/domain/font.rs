//! Handles into the host-owned font store, positioned glyphs, and (v0.5 B3) the
//! rasterized product a [`crate::application::FontProvider`] hands back.
//!
//! A display list never carries font *bytes* or glyph *outlines* — it carries
//! identifiers the backend resolves through a `FontProvider`. That is what keeps
//! `DisplayList` cheap to clone, cheap to serialize, and free of any dependency
//! on how fonts were discovered.

use core::fmt;

use crate::domain::geometry::Point;
use crate::domain::unit::Au;

/// A face already resolved and registered with the host's font store.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontId(u16);

impl FontId {
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl fmt::Display for FontId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "font #{}", self.0)
    }
}

/// A glyph index within one face — *not* a character.
///
/// The distinction matters: the mapping from character to glyph is the face's
/// `cmap`, and it is neither injective nor stable across faces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GlyphId(u16);

impl GlyphId {
    /// The glyph every face reserves for "no mapping" — index 0, `.notdef`.
    pub const NOTDEF: Self = Self(0);

    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl fmt::Display for GlyphId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "glyph #{}", self.0)
    }
}

/// One glyph, already positioned by whoever laid the text out.
///
/// Positioning happens before the display list, not inside the backend: the
/// backend's job is to rasterize, and keeping shaping out of it is what lets the
/// software and GPU backends agree pixel for pixel (`PRD-005:62-63`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphInstance {
    glyph: GlyphId,
    position: Point,
}

impl GlyphInstance {
    #[must_use]
    pub const fn new(glyph: GlyphId, position: Point) -> Self {
        Self { glyph, position }
    }

    #[must_use]
    pub const fn glyph(self) -> GlyphId {
        self.glyph
    }

    /// The pen position of this glyph, on the text baseline.
    #[must_use]
    pub const fn position(self) -> Point {
        self.position
    }
}

impl fmt::Display for GlyphInstance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} at {}", self.glyph, self.position)
    }
}

/// A first-class collection of positioned glyphs — one `DrawText` payload.
///
/// A collection type rather than a bare `Vec` (`ADR-0010` rule 3): a run is
/// built once and read many times, and giving it a name is what lets the
/// backend take `&GlyphRun` without exposing a mutable vector.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct GlyphRun {
    glyphs: Vec<GlyphInstance>,
}

impl GlyphRun {
    #[must_use]
    pub const fn new() -> Self {
        Self { glyphs: Vec::new() }
    }

    /// Collects a run from positioned glyphs.
    #[must_use]
    pub fn from_glyphs(glyphs: impl IntoIterator<Item = GlyphInstance>) -> Self {
        Self {
            glyphs: glyphs.into_iter().collect(),
        }
    }

    pub fn push(&mut self, glyph: GlyphInstance) {
        self.glyphs.push(glyph);
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.glyphs.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &GlyphInstance> + '_ {
        self.glyphs.iter()
    }
}

impl<'run> IntoIterator for &'run GlyphRun {
    type Item = &'run GlyphInstance;
    type IntoIter = core::slice::Iter<'run, GlyphInstance>;

    fn into_iter(self) -> Self::IntoIter {
        self.glyphs.iter()
    }
}

/// The vertical measurements of a face at one size — `PRD-005` B3.
///
/// `ascent` and `descent` are both non-negative magnitudes (distance above and
/// below the baseline, respectively), matching how every font format reports
/// them; a caller wanting a signed offset negates `descent` itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FaceMetrics {
    ascent: Au,
    descent: Au,
    line_gap: Au,
}

impl FaceMetrics {
    #[must_use]
    pub const fn new(ascent: Au, descent: Au, line_gap: Au) -> Self {
        Self {
            ascent,
            descent,
            line_gap,
        }
    }

    #[must_use]
    pub const fn ascent(self) -> Au {
        self.ascent
    }

    #[must_use]
    pub const fn descent(self) -> Au {
        self.descent
    }

    #[must_use]
    pub const fn line_gap(self) -> Au {
        self.line_gap
    }

    /// `ascent + descent + line_gap`, saturating at the `Au` extremes.
    #[must_use]
    pub const fn line_height(self) -> Au {
        self.ascent
            .saturating_add(self.descent)
            .saturating_add(self.line_gap)
    }
}

/// A rasterized glyph: a rectangular coverage mask plus the offset from the
/// glyph's pen position (on the text baseline) to the mask's top-left corner.
///
/// A first-class collection over the coverage bytes (`ADR-0010` rule 3) rather
/// than a bare `Vec<u8>`, so its own invariant — `coverage.len() ==
/// width * height` — cannot be broken by a caller. Every [`FontProvider`] gives
/// the backend one of these per glyph; the backend only ever blits it, never
/// walks a curve (`PRD-005:62-63` keeps shaping and rasterizing off the
/// backend's frozen contract).
///
/// [`FontProvider`]: crate::application::FontProvider
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlyphBitmap {
    width: u32,
    height: u32,
    coverage: Vec<u8>,
    bearing: Point,
}

impl GlyphBitmap {
    /// A mask with nothing to paint — a space, or a glyph this provider does
    /// not have. Rasterizing to nothing is not an error (`Result::Ok`); it is
    /// the correct answer for whitespace.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
            coverage: Vec::new(),
            bearing: Point::ORIGIN,
        }
    }

    /// Builds a mask, refusing a `coverage` length that does not match
    /// `width * height` — a provider bug, not something a backend should paint
    /// around.
    #[must_use]
    pub fn new(width: u32, height: u32, coverage: Vec<u8>, bearing: Point) -> Option<Self> {
        let cell_count = usize::try_from(width)
            .ok()?
            .checked_mul(usize::try_from(height).ok()?)?;
        if coverage.len() != cell_count {
            return None;
        }
        Some(Self {
            width,
            height,
            coverage,
            bearing,
        })
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// The offset from the glyph's pen position to this mask's top-left pixel.
    #[must_use]
    pub const fn bearing(&self) -> Point {
        self.bearing
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// The coverage at `(column, row)`, `0` outside the mask — never a panic on
    /// an out-of-range query.
    #[must_use]
    pub fn coverage_at(&self, column: u32, row: u32) -> u8 {
        if column >= self.width || row >= self.height {
            return 0;
        }
        let Ok(width) = usize::try_from(self.width) else {
            return 0;
        };
        let Ok(row) = usize::try_from(row) else {
            return 0;
        };
        let Ok(column) = usize::try_from(column) else {
            return 0;
        };
        let Some(index) = row
            .checked_mul(width)
            .and_then(|base| base.checked_add(column))
        else {
            return 0;
        };
        self.coverage.get(index).copied().unwrap_or(0)
    }
}
