//! Value object representing an HTML tag name.

use crate::domain::error::HtmlError;
use crate::domain::location::SourceLocation;
use core::fmt;

/// A validated, normalized (lowercased) HTML tag name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TagName(String);

impl TagName {
    /// Create and validate a tag name from an input string slice.
    pub fn new(raw_name: impl Into<String>, location: SourceLocation) -> Result<Self, HtmlError> {
        let normalized = raw_name.into().to_ascii_lowercase();
        if normalized.is_empty() {
            return Err(HtmlError::invalid_tag("tag name cannot be empty", location));
        }
        Ok(Self(normalized))
    }

    /// Create a tag name without validation (for trusted internal constants).
    #[must_use]
    pub fn new_unchecked(name: impl Into<String>) -> Self {
        Self(name.into().to_ascii_lowercase())
    }

    /// Access tag name as string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into inner string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for TagName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl AsRef<str> for TagName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for TagName {
    fn eq(&self, other: &str) -> bool {
        self.0.eq_ignore_ascii_case(other)
    }
}

impl PartialEq<&str> for TagName {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq_ignore_ascii_case(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_name_lowercases_and_validates() {
        let location = SourceLocation::initial();
        let tag = TagName::new("DIV", location).expect("valid tag");
        assert_eq!(tag.as_str(), "div");
        assert_eq!(tag, "div");
    }

    #[test]
    fn empty_tag_name_fails() {
        let location = SourceLocation::initial();
        let error = TagName::new("", location).unwrap_err();
        assert!(matches!(error, HtmlError::InvalidTag { .. }));
    }

    #[test]
    fn display_and_into_string() {
        let tag = TagName::new_unchecked("span");
        assert_eq!(tag.to_string(), "span");
        assert_eq!(tag.into_string(), "span");
    }
}
