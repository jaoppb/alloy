//! [`ParseNote`] — the typed record of something the parser **recovered** from.
//!
//! CSS Syntax Level 3 §5.4 makes recovery mandatory: a malformed rule is
//! consumed up to the `}` that closes it and parsing continues. Recovering
//! silently, though, is how a declared cut shrinks without anyone noticing
//! (`relatório §2.8:350-354`). Every recovery — an unknown at-rule, a dropped
//! declaration, a selector outside the v0.5 cut, an unterminated string —
//! leaves a note carrying the [`SourceSpan`] where it happened, and
//! [`crate::StyleSheetSet::notes`] is where the manifest runner and any
//! diagnostic read them back.

use core::fmt;

use crate::domain::error::SourceSpan;

/// One recovered construct: what was skipped, and where.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ParseNote {
    message: String,
    span: SourceSpan,
}

impl ParseNote {
    #[must_use]
    pub fn new(message: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

impl fmt::Display for ParseNote {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.span, self.message)
    }
}

/// Every note raised while parsing one stylesheet, in source order. A
/// first-class collection — no public `Vec`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ParseNotes {
    notes: Vec<ParseNote>,
}

impl ParseNotes {
    #[must_use]
    pub const fn new() -> Self {
        Self { notes: Vec::new() }
    }

    pub fn push(&mut self, note: ParseNote) {
        self.notes.push(note);
    }

    /// Appends every note of `other`, keeping both source orders — used when
    /// one document's several `<style>` elements merge into one set.
    pub fn absorb(&mut self, other: Self) {
        self.notes.extend(other.notes);
    }

    pub fn iter(&self) -> impl Iterator<Item = &ParseNote> + '_ {
        self.notes.iter()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.notes.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }
}
