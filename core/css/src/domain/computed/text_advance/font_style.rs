//! [`FontStyle`] — font face style selection (CSS Fonts L4 §2.3).

use core::fmt;

/// The style of the font face: normal, italic, or oblique.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FontStyle {
    /// Normal / upright font face — the CSS `initial` value.
    #[default]
    Normal,
    /// Italic font face.
    Italic,
    /// Obliqued / slanted font face.
    Oblique,
}

impl FontStyle {
    /// Whether this font style is italic.
    #[must_use]
    pub const fn is_italic(self) -> bool {
        matches!(self, Self::Italic)
    }

    /// Whether this font style is oblique.
    #[must_use]
    pub const fn is_oblique(self) -> bool {
        matches!(self, Self::Oblique)
    }

    /// The keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Italic => "italic",
            Self::Oblique => "oblique",
        }
    }
}

impl fmt::Display for FontStyle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}
