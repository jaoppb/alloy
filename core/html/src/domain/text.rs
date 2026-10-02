//! Value object representing text content in HTML tokens.

use core::fmt;

/// A validated text value object for character tokens and comments.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Text(String);

impl Text {
    /// Create a new text value object.
    #[must_use]
    pub fn new(content: impl Into<String>) -> Self {
        Self(content.into())
    }

    /// Access text content as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into inner string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }

    /// Checks whether the text is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Number of bytes in the text.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Append a character to this text.
    pub fn push(&mut self, character: char) {
        self.0.push(character);
    }

    /// Append a string slice to this text.
    pub fn push_str(&mut self, slice: &str) {
        self.0.push_str(slice);
    }
}

impl fmt::Display for Text {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl From<&str> for Text {
    fn from(source: &str) -> Self {
        Self::new(source)
    }
}

impl From<String> for Text {
    fn from(source: String) -> Self {
        Self(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_construction_and_access() {
        let text = Text::new("hello world");
        assert_eq!(text.as_str(), "hello world");
        assert_eq!(text.len(), 11);
        assert!(!text.is_empty());
    }

    #[test]
    fn text_mutations() {
        let mut text = Text::new("hello");
        text.push(' ');
        text.push_str("world");
        assert_eq!(text.into_string(), "hello world");
    }

    #[test]
    fn text_display_matches_inner() {
        let text = Text::new("sample");
        assert_eq!(text.to_string(), "sample");
    }
}
