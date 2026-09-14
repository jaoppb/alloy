//! The value objects of [`crate::TextMeasurer`]: a run of text to measure, the
//! computed text style it is measured under, and the metrics that come back.
//!
//! `TextMeasurer` is consumed by the layout engine's inline formatting context
//! from B4; B0 defines the vocabulary and a monospace reference implementation
//! so the port is born whole.

use graphics::Au;

/// A run of text handed to a measurer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextRun {
    text: String,
}

impl TextRun {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The number of Unicode scalar values — the unit a fixed-advance measurer
    /// counts.
    #[must_use]
    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

/// The subset of computed style a measurer needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ComputedText {
    font_size: Au,
}

impl ComputedText {
    #[must_use]
    pub const fn new(font_size: Au) -> Self {
        Self { font_size }
    }

    #[must_use]
    pub const fn font_size(self) -> Au {
        self.font_size
    }
}

/// The measured extent of a text run: one line box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextMetrics {
    width: Au,
    height: Au,
}

impl TextMetrics {
    #[must_use]
    pub const fn new(width: Au, height: Au) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn width(self) -> Au {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> Au {
        self.height
    }
}
