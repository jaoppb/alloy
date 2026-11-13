//! The name a document gives the window it is shown in.

use std::fmt;

use network::Url;

/// A document's title as WHATWG `document.title` defines it (§4.2.2): the
/// title text with ASCII whitespace stripped and collapsed — or, for a page
/// with no usable title, the URL it was loaded from (#84).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentTitle(String);

impl DocumentTitle {
    /// `text` with runs of ASCII whitespace collapsed to one space and the
    /// ends stripped; `None` when nothing but whitespace is left, so a blank
    /// `<title>` does not name the window.
    #[must_use]
    pub fn collapsed(text: &str) -> Option<Self> {
        let words: Vec<&str> = text.split_ascii_whitespace().collect();
        if words.is_empty() {
            return None;
        }
        Some(Self(words.join(" ")))
    }

    /// The title of a page that declares none: the URL it was loaded from.
    #[must_use]
    pub fn from_url(url: &Url) -> Self {
        Self(url.to_string())
    }

    /// The title text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DocumentTitle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_whitespace_is_stripped_and_collapsed() {
        let title = DocumentTitle::collapsed("\n\t Example \x0c\r\n Domain  ");

        assert_eq!(
            title.map(|title| title.to_string()).as_deref(),
            Some("Example Domain")
        );
    }

    #[test]
    fn non_ascii_whitespace_is_kept() {
        let title = DocumentTitle::collapsed("a\u{a0}b");

        assert_eq!(title.as_ref().map(DocumentTitle::as_str), Some("a\u{a0}b"));
    }

    #[test]
    fn whitespace_only_text_is_no_title() {
        assert_eq!(DocumentTitle::collapsed(" \t\n "), None);
    }
}
