//! [`TextAdvanceStyle`] — advanced typography and line formatting aggregate.

use crate::domain::computed::text_advance::decoration::TextDecoration;
use crate::domain::computed::text_advance::font_style::FontStyle;
use crate::domain::computed::text_advance::font_weight::FontWeight;
use crate::domain::computed::text_advance::line_height::LineHeight;
use crate::domain::computed::text_advance::overflow::TextOverflow;
use crate::domain::computed::text_advance::spacing::{LetterSpacing, WordSpacing};
use crate::domain::computed::text_advance::transform::TextTransform;
use crate::domain::computed::text_advance::wrap::{OverflowWrap, WordBreak};

/// Composite styling for typography and line layout.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TextAdvanceStyle {
    font_weight: FontWeight,
    font_style: FontStyle,
    line_height: LineHeight,
    letter_spacing: LetterSpacing,
    word_spacing: WordSpacing,
    text_decoration: TextDecoration,
    text_transform: TextTransform,
    text_overflow: TextOverflow,
    overflow_wrap: OverflowWrap,
    word_break: WordBreak,
}

impl TextAdvanceStyle {
    /// All text advance properties at their CSS `initial` values.
    #[must_use]
    pub const fn initial() -> Self {
        Self {
            font_weight: FontWeight::NORMAL,
            font_style: FontStyle::Normal,
            line_height: LineHeight::Normal,
            letter_spacing: LetterSpacing::Normal,
            word_spacing: WordSpacing::Normal,
            text_decoration: TextDecoration::initial(),
            text_transform: TextTransform::None,
            text_overflow: TextOverflow::Clip,
            overflow_wrap: OverflowWrap::Normal,
            word_break: WordBreak::Normal,
        }
    }

    /// Inherits CSS inheritable properties from `parent`, resetting non-inheritable ones.
    #[must_use]
    pub const fn inheriting_from(parent: &Self) -> Self {
        Self {
            font_weight: parent.font_weight,
            font_style: parent.font_style,
            line_height: parent.line_height,
            letter_spacing: parent.letter_spacing,
            word_spacing: parent.word_spacing,
            text_decoration: TextDecoration::initial(),
            text_transform: parent.text_transform,
            text_overflow: TextOverflow::Clip,
            overflow_wrap: parent.overflow_wrap,
            word_break: parent.word_break,
        }
    }

    #[must_use]
    pub const fn font_weight(self) -> FontWeight {
        self.font_weight
    }

    #[must_use]
    pub const fn font_style(self) -> FontStyle {
        self.font_style
    }

    #[must_use]
    pub const fn line_height(self) -> LineHeight {
        self.line_height
    }

    #[must_use]
    pub const fn letter_spacing(self) -> LetterSpacing {
        self.letter_spacing
    }

    #[must_use]
    pub const fn word_spacing(self) -> WordSpacing {
        self.word_spacing
    }

    #[must_use]
    pub const fn text_decoration(self) -> TextDecoration {
        self.text_decoration
    }

    #[must_use]
    pub const fn text_transform(self) -> TextTransform {
        self.text_transform
    }

    #[must_use]
    pub const fn text_overflow(self) -> TextOverflow {
        self.text_overflow
    }

    #[must_use]
    pub const fn overflow_wrap(self) -> OverflowWrap {
        self.overflow_wrap
    }

    #[must_use]
    pub const fn word_break(self) -> WordBreak {
        self.word_break
    }

    #[must_use]
    pub const fn with_font_weight(self, font_weight: FontWeight) -> Self {
        Self {
            font_weight,
            ..self
        }
    }

    #[must_use]
    pub const fn with_font_style(self, font_style: FontStyle) -> Self {
        Self { font_style, ..self }
    }

    #[must_use]
    pub const fn with_line_height(self, line_height: LineHeight) -> Self {
        Self {
            line_height,
            ..self
        }
    }

    #[must_use]
    pub const fn with_letter_spacing(self, letter_spacing: LetterSpacing) -> Self {
        Self {
            letter_spacing,
            ..self
        }
    }

    #[must_use]
    pub const fn with_word_spacing(self, word_spacing: WordSpacing) -> Self {
        Self {
            word_spacing,
            ..self
        }
    }

    #[must_use]
    pub const fn with_text_decoration(self, text_decoration: TextDecoration) -> Self {
        Self {
            text_decoration,
            ..self
        }
    }

    #[must_use]
    pub const fn with_text_transform(self, text_transform: TextTransform) -> Self {
        Self {
            text_transform,
            ..self
        }
    }

    #[must_use]
    pub const fn with_text_overflow(self, text_overflow: TextOverflow) -> Self {
        Self {
            text_overflow,
            ..self
        }
    }

    #[must_use]
    pub const fn with_overflow_wrap(self, overflow_wrap: OverflowWrap) -> Self {
        Self {
            overflow_wrap,
            ..self
        }
    }

    #[must_use]
    pub const fn with_word_break(self, word_break: WordBreak) -> Self {
        Self { word_break, ..self }
    }
}

impl Default for TextAdvanceStyle {
    fn default() -> Self {
        Self::initial()
    }
}
