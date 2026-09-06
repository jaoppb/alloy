//! Grid item placement on grid lines (CSS Grid L1 §8).

use core::fmt;

/// A 1-based signed grid line index (e.g. `1`, `3`, `-1`). `0` is invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridLine(i32);

impl GridLine {
    /// Constructs a grid line, or `None` if `0`.
    #[must_use]
    pub const fn new(line: i32) -> Option<Self> {
        if line == 0 {
            return None;
        }
        Some(Self(line))
    }

    /// The signed 1-based index.
    #[must_use]
    pub const fn as_i32(self) -> i32 {
        self.0
    }
}

impl fmt::Display for GridLine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// A positive span count across tracks (`span N`). `0` is invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridSpan(u32);

impl GridSpan {
    /// Default unit span `span 1`.
    pub const ONE: Self = Self(1);

    /// Constructs a span, or `None` if `0`.
    #[must_use]
    pub const fn new(count: u32) -> Option<Self> {
        if count == 0 {
            return None;
        }
        Some(Self(count))
    }

    /// The span count.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

impl fmt::Display for GridSpan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "span {}", self.0)
    }
}

/// Identifier for a named grid line.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridLineName {
    bytes: [u8; Self::CAPACITY],
    len: u8,
}

impl GridLineName {
    /// Maximum identifier length in bytes.
    pub const CAPACITY: usize = 32;

    /// Validates and constructs a named grid line identifier.
    #[must_use]
    pub fn new(name: impl AsRef<str>) -> Option<Self> {
        let text = name.as_ref().trim();
        let is_reserved = text.is_empty()
            || text.eq_ignore_ascii_case("auto")
            || text.eq_ignore_ascii_case("span");
        if is_reserved || text.len() > Self::CAPACITY {
            return None;
        }
        let mut bytes = [0u8; Self::CAPACITY];
        let lower = text.to_ascii_lowercase();
        let slice = bytes.get_mut(..lower.len())?;
        slice.copy_from_slice(lower.as_bytes());
        let len = u8::try_from(lower.len()).ok()?;
        Some(Self { bytes, len })
    }

    /// The line name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        let len = usize::from(self.len);
        let Some(slice) = self.bytes.get(..len) else {
            return "";
        };
        core::str::from_utf8(slice).unwrap_or("")
    }
}

impl fmt::Debug for GridLineName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "GridLineName({:?})", self.as_str())
    }
}

impl fmt::Display for GridLineName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A grid placement value for one edge of a grid item (CSS Grid L1 §8.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GridPlacement {
    /// Auto placement.
    #[default]
    Auto,
    /// Placement at a numeric line index.
    Line(GridLine),
    /// Placement spanning N tracks.
    Span(GridSpan),
    /// Placement at the first matching named line.
    Named(GridLineName),
    /// Placement at numeric line with name.
    LineNamed(GridLine, GridLineName),
    /// Placement spanning N tracks with named target.
    SpanNamed(GridSpan, GridLineName),
}

impl GridPlacement {
    #[must_use]
    pub const fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }

    #[must_use]
    pub const fn is_span(&self) -> bool {
        matches!(self, Self::Span(..) | Self::SpanNamed(..))
    }

    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        GridLineName::new(name).map(Self::Named)
    }

    #[must_use]
    pub const fn line(index: GridLine) -> Self {
        Self::Line(index)
    }

    #[must_use]
    pub const fn span(count: GridSpan) -> Self {
        Self::Span(count)
    }
}

impl fmt::Display for GridPlacement {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => formatter.write_str("auto"),
            Self::Line(line) => write!(formatter, "{line}"),
            Self::Span(span) => write!(formatter, "{span}"),
            Self::Named(name) => write!(formatter, "{name}"),
            Self::LineNamed(line, name) => write!(formatter, "{line} {name}"),
            Self::SpanNamed(span, name) => write!(formatter, "{span} {name}"),
        }
    }
}
