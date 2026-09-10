//! [`TextOverflow`] — text overflow indication (CSS Text Decoration L3 §4).

use core::fmt;

/// How overflowing inline content is rendered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextOverflow {
    /// Clip text at the overflow edge — CSS `initial`.
    #[default]
    Clip,
    /// Render an ellipsis (`…`) to indicate clipped text.
    Ellipsis,
}

impl TextOverflow {
    #[must_use]
    pub const fn is_ellipsis(self) -> bool {
        matches!(self, Self::Ellipsis)
    }

    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Clip => "clip",
            Self::Ellipsis => "ellipsis",
        }
    }
}

impl fmt::Display for TextOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}
