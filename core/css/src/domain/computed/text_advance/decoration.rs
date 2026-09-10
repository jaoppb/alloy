//! Text decoration properties (CSS Text Decoration L3).

use core::fmt;

use crate::domain::color::CssColor;

/// Decorative lines added to text (CSS Text Decoration L3 §2.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextDecorationLine {
    underline: bool,
    overline: bool,
    line_through: bool,
}

impl TextDecorationLine {
    /// No decoration line — CSS `initial`.
    pub const NONE: Self = Self {
        underline: false,
        overline: false,
        line_through: false,
    };
    /// Underline decoration.
    pub const UNDERLINE: Self = Self {
        underline: true,
        overline: false,
        line_through: false,
    };
    /// Overline decoration.
    pub const OVERLINE: Self = Self {
        underline: false,
        overline: true,
        line_through: false,
    };
    /// Line-through / strikethrough decoration.
    pub const LINE_THROUGH: Self = Self {
        underline: false,
        overline: false,
        line_through: true,
    };

    #[must_use]
    pub const fn is_none(self) -> bool {
        !self.underline && !self.overline && !self.line_through
    }

    #[must_use]
    pub const fn has_underline(self) -> bool {
        self.underline
    }

    #[must_use]
    pub const fn has_overline(self) -> bool {
        self.overline
    }

    #[must_use]
    pub const fn has_line_through(self) -> bool {
        self.line_through
    }

    #[must_use]
    pub const fn with_underline(self, underline: bool) -> Self {
        Self { underline, ..self }
    }

    #[must_use]
    pub const fn with_overline(self, overline: bool) -> Self {
        Self { overline, ..self }
    }

    #[must_use]
    pub const fn with_line_through(self, line_through: bool) -> Self {
        Self {
            line_through,
            ..self
        }
    }
}

fn write_separated(
    formatter: &mut fmt::Formatter<'_>,
    first: &mut bool,
    text: &str,
) -> fmt::Result {
    if !*first {
        formatter.write_str(" ")?;
    }
    *first = false;
    formatter.write_str(text)
}

impl fmt::Display for TextDecorationLine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_none() {
            return formatter.write_str("none");
        }
        let mut first = true;
        if self.underline {
            write_separated(formatter, &mut first, "underline")?;
        }
        if self.overline {
            write_separated(formatter, &mut first, "overline")?;
        }
        if self.line_through {
            write_separated(formatter, &mut first, "line-through")?;
        }
        Ok(())
    }
}

/// Visual stroke style for text decoration lines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextDecorationStyle {
    #[default]
    Solid,
    Double,
    Dotted,
    Dashed,
    Wavy,
}

impl TextDecorationStyle {
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Solid => "solid",
            Self::Double => "double",
            Self::Dotted => "dotted",
            Self::Dashed => "dashed",
            Self::Wavy => "wavy",
        }
    }
}

impl fmt::Display for TextDecorationStyle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}

/// The composite text decoration style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextDecoration {
    line: TextDecorationLine,
    color: CssColor,
    style: TextDecorationStyle,
}

impl TextDecoration {
    #[must_use]
    pub const fn initial() -> Self {
        Self {
            line: TextDecorationLine::NONE,
            color: CssColor::BLACK,
            style: TextDecorationStyle::Solid,
        }
    }

    #[must_use]
    pub const fn line(self) -> TextDecorationLine {
        self.line
    }

    #[must_use]
    pub const fn color(self) -> CssColor {
        self.color
    }

    #[must_use]
    pub const fn style(self) -> TextDecorationStyle {
        self.style
    }

    #[must_use]
    pub const fn with_line(self, line: TextDecorationLine) -> Self {
        Self { line, ..self }
    }

    #[must_use]
    pub const fn with_color(self, color: CssColor) -> Self {
        Self { color, ..self }
    }

    #[must_use]
    pub const fn with_style(self, style: TextDecorationStyle) -> Self {
        Self { style, ..self }
    }
}

impl Default for TextDecoration {
    fn default() -> Self {
        Self::initial()
    }
}

impl fmt::Display for TextDecoration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {} {}", self.line, self.style, self.color)
    }
}
