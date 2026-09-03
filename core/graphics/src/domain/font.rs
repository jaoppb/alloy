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
