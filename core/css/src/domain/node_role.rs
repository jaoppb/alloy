//! [`NodeRole`] — what an element is to layout beyond its computed style.
//!
//! Layout is handed only a [`crate::StyledTree`] (`PRD-007:56-60`), which
//! carries no tag name, so an element whose rendering the HTML rendering rules
//! fix outside CSS has to say so inside the aggregate. A `<br>` is the one such
//! element today (issue #82).

use html::TagName;

/// An element's role in layout, decided by its tag when the styled tree is
/// built.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NodeRole {
    /// Laid out by its computed style alone.
    #[default]
    Ordinary,
    /// A `<br>`: a forced line break inside the inline formatting context
    /// (WHATWG HTML §15.3.4, `br { display-outside: newline }`).
    LineBreak,
}

impl NodeRole {
    /// The role an element with `tag` plays; every non-element node is
    /// [`NodeRole::Ordinary`].
    #[must_use]
    pub const fn for_tag(tag: Option<&TagName>) -> Self {
        match tag {
            Some(TagName::Br) => Self::LineBreak,
            _ => Self::Ordinary,
        }
    }

    /// Whether this node forces a line break.
    #[must_use]
    pub const fn is_line_break(self) -> bool {
        matches!(self, Self::LineBreak)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_br_is_a_line_break() {
        assert_eq!(NodeRole::for_tag(Some(&TagName::Br)), NodeRole::LineBreak);
        assert_eq!(NodeRole::for_tag(Some(&TagName::Span)), NodeRole::Ordinary);
        assert_eq!(NodeRole::for_tag(None), NodeRole::Ordinary);
        assert!(NodeRole::LineBreak.is_line_break());
        assert!(!NodeRole::Ordinary.is_line_break());
    }
}
