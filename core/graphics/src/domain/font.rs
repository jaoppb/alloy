//! Handles into the host-owned font store, and positioned glyphs.
//!
//! A display list never carries font *bytes* or glyph *outlines* — it carries
//! identifiers the backend resolves through a `FontProvider`. That is what keeps
//! `DisplayList` cheap to clone, cheap to serialize, and free of any dependency
//! on how fonts were discovered.

use core::fmt;

use crate::domain::geometry::Point;

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
