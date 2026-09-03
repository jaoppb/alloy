//! Vector outlines, for the `DrawPath` command of `PRD-005:68`.
//!
//! `PRD-005` names `Path2D` and `Stroke` without specifying either, so this is
//! the minimal shape that makes the frozen contract *real* rather than a
//! placeholder: a sequence of segments in [`Au`], plus a stroke description.
//! The v0.3 software backend refuses `DrawPath` with
//! [`crate::GraphicsError::Unsupported`] — the contract is born whole and the
//! implementation arrives incrementally (v0.3 report §2.3).

use crate::domain::color::Color;
use crate::domain::geometry::Point;
use crate::domain::unit::Au;

/// One step of an outline. Curves are cubic and quadratic Béziers only, which
/// is what both TrueType and PostScript outlines reduce to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PathSegment {
    MoveTo {
        to: Point,
    },
    LineTo {
        to: Point,
    },
    QuadraticTo {
        control: Point,
        to: Point,
    },
    CubicTo {
        first_control: Point,
        second_control: Point,
        to: Point,
    },
    Close,
}

/// A first-class collection of outline segments.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Path {
    segments: Vec<PathSegment>,
}

impl Path {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    #[must_use]
    pub fn from_segments(segments: impl IntoIterator<Item = PathSegment>) -> Self {
        Self {
            segments: segments.into_iter().collect(),
        }
    }

    pub fn push(&mut self, segment: PathSegment) {
        self.segments.push(segment);
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.segments.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &PathSegment> + '_ {
        self.segments.iter()
    }
}

/// How an outline is stroked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Stroke {
    width: Au,
    color: Color,
}

impl Stroke {
    #[must_use]
    pub const fn new(width: Au, color: Color) -> Self {
        Self { width, color }
    }

    #[must_use]
    pub const fn width(self) -> Au {
        self.width
    }

    #[must_use]
    pub const fn color(self) -> Color {
        self.color
    }
}
