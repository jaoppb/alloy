//! Vocabulary of element namespaces (WHATWG HTML §13.2.6.5).

/// The XML namespace an element belongs to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Namespace {
    /// The HTML namespace (`http://www.w3.org/1999/xhtml`).
    #[default]
    Html,
    /// The SVG namespace (`http://www.w3.org/2000/svg`).
    Svg,
    /// The `MathML` namespace (`http://www.w3.org/1998/Math/MathML`).
    MathMl,
}

impl Namespace {
    /// Whether this namespace is foreign (`SVG` or `MathML`).
    #[must_use]
    pub const fn is_foreign(self) -> bool {
        match self {
            Self::Html => false,
            Self::Svg | Self::MathMl => true,
        }
    }

    /// The namespace identifier as a static string slice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Svg => "svg",
            Self::MathMl => "mathml",
        }
    }
}
