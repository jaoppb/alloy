//! Recoverable parse diagnostics (ADR-0023, issue #36).
//!
//! A diagnostic reports a malformation the parser recovered from. It is data delivered through the
//! port ([`crate::TreeSink::parse_error`]), never an `Err`: only [`crate::HtmlError`] aborts a parse.

use crate::domain::location::SourceLocation;
use core::fmt;

macro_rules! parse_error_codes {
    ($($(#[$meta:meta])* $variant:ident => $code:literal,)+) => {
        /// A closed vocabulary of recoverable parse conditions.
        ///
        /// Names follow the WHATWG HTML §13.2.2 parse-error codes where one exists; the rest are
        /// tree-builder or adapter conditions this implementation reports itself.
        #[non_exhaustive]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum ParseErrorCode {
            $($(#[$meta])* $variant,)+
        }

        impl ParseErrorCode {
            /// Every code, as its kebab-case spelling — the registry the manifest gate checks.
            pub const CODES: &'static [&'static str] = &[$($code),+];

            /// The kebab-case spelling of this code.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $code,)+
                }
            }
        }
    };
}

parse_error_codes! {
    /// `"`, `'` or `<` inside an attribute name (§13.2.5.33); the attribute is kept.
    UnexpectedCharacterInAttributeName => "unexpected-character-in-attribute-name",
    /// `=` where an attribute name should start; it becomes the name's first character.
    UnexpectedEqualsSignBeforeAttributeName => "unexpected-equals-sign-before-attribute-name",
    /// A second attribute with the same name; the first wins and the later one is dropped.
    DuplicateAttribute => "duplicate-attribute",
    /// `name=` followed directly by `>`; the attribute gets an empty value.
    MissingAttributeValue => "missing-attribute-value",
    /// A quoted value followed directly by another attribute name.
    MissingWhitespaceBetweenAttributes => "missing-whitespace-between-attributes",
    /// A `/` inside a tag that is not part of `/>`.
    UnexpectedSolidusInTag => "unexpected-solidus-in-tag",
    /// An end tag carrying attributes; they are dropped.
    EndTagWithAttributes => "end-tag-with-attributes",
    /// End of input inside a tag; the unfinished tag is dropped.
    EofInTag => "eof-in-tag",
    /// `</>`; nothing is emitted.
    MissingEndTagName => "missing-end-tag-name",
    /// End of input right after `<` or `</`; the characters are emitted as text.
    EofBeforeTagName => "eof-before-tag-name",
    /// A non-letter after `<` or `</`.
    InvalidFirstCharacterOfTagName => "invalid-first-character-of-tag-name",
    /// `<?`; treated as a bogus comment.
    UnexpectedQuestionMarkInsteadOfTagName => "unexpected-question-mark-instead-of-tag-name",
    /// `&name;` that names no known character reference.
    UnknownNamedCharacterReference => "unknown-named-character-reference",
    /// `&#` without digits.
    AbsenceOfDigitsInNumericCharacterReference => "absence-of-digits-in-numeric-character-reference",
    /// `&#0;`.
    NullCharacterReference => "null-character-reference",
    /// A numeric reference above U+10FFFF.
    CharacterReferenceOutsideUnicodeRange => "character-reference-outside-unicode-range",
    /// A numeric reference to a surrogate.
    SurrogateCharacterReference => "surrogate-character-reference",
    /// A NUL in the input stream.
    UnexpectedNullCharacter => "unexpected-null-character",
    /// A control character other than whitespace in the input stream.
    ControlCharacterInInputStream => "control-character-in-input-stream",
    /// `<!-->` or `<!--->`.
    AbruptClosingOfEmptyComment => "abrupt-closing-of-empty-comment",
    /// `<!` that opens neither a comment nor a doctype; treated as a bogus comment.
    IncorrectlyOpenedComment => "incorrectly-opened-comment",
    /// End of input inside a comment.
    EofInComment => "eof-in-comment",
    /// End of input inside a doctype.
    EofInDoctype => "eof-in-doctype",
    /// `<!DOCTYPE>` without a name.
    MissingDoctypeName => "missing-doctype-name",
    /// A tag name `TagName` rejects (not `[A-Za-z][A-Za-z0-9-]*`); the tag is dropped.
    InvalidTagName => "invalid-tag-name",
    /// An attribute name that breaks the strict `AttributeName` rule; the attribute is dropped
    /// (ADR-0024).
    InvalidAttributeName => "invalid-attribute-name",
    /// An end tag with no matching open element.
    StrayEndTag => "stray-end-tag",
    /// An end tag closing over an open element that is not spec-implied.
    EndTagDoesNotMatchCurrentNode => "end-tag-does-not-match-current-node",
    /// A start tag whose omission rule closed a non-implied open element.
    ElementClosedImplicitly => "element-closed-implicitly",
    /// A repeated `<html>` start tag; its attributes are merged.
    UnexpectedHtmlStartTag => "unexpected-html-start-tag",
    /// A repeated `<body>` start tag; its attributes are merged.
    UnexpectedBodyStartTag => "unexpected-body-start-tag",
    /// A repeated `<head>` start tag; ignored.
    UnexpectedHeadStartTag => "unexpected-head-start-tag",
    /// A doctype that forces quirks mode (missing or non-`html` name).
    QuirksModeDoctype => "quirks-mode-doctype",
}

impl fmt::Display for ParseErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A recoverable parse error: what happened and where.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{code} at {location}")]
pub struct ParseDiagnostic {
    code: ParseErrorCode,
    location: SourceLocation,
}

impl ParseDiagnostic {
    /// Creates a diagnostic.
    #[must_use]
    pub const fn new(code: ParseErrorCode, location: SourceLocation) -> Self {
        Self { code, location }
    }

    /// What happened.
    #[must_use]
    pub const fn code(&self) -> ParseErrorCode {
        self.code
    }

    /// Where it happened.
    #[must_use]
    pub const fn location(&self) -> SourceLocation {
        self.location
    }
}

/// An ordered, first-class collection of [`ParseDiagnostic`]s, in detection order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Diagnostics {
    entries: Vec<ParseDiagnostic>,
}

impl Diagnostics {
    /// Creates an empty collection.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Appends a diagnostic.
    pub fn push(&mut self, diagnostic: ParseDiagnostic) {
        self.entries.push(diagnostic);
    }

    /// Number of diagnostics.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing was reported.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The first diagnostic reported, if any.
    #[must_use]
    pub fn first(&self) -> Option<&ParseDiagnostic> {
        self.entries.first()
    }

    /// The diagnostics as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[ParseDiagnostic] {
        &self.entries
    }

    /// Iterator over the diagnostics.
    pub fn iter(&self) -> core::slice::Iter<'_, ParseDiagnostic> {
        self.entries.iter()
    }
}

impl<'a> IntoIterator for &'a Diagnostics {
    type Item = &'a ParseDiagnostic;
    type IntoIter = core::slice::Iter<'a, ParseDiagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_a_unique_kebab_case_spelling() {
        let mut seen = std::collections::BTreeSet::new();
        for code in ParseErrorCode::CODES {
            assert!(seen.insert(*code), "duplicate code {code}");
            assert!(code.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
        }
    }

    #[test]
    fn a_diagnostic_displays_its_code_and_location() {
        let diagnostic =
            ParseDiagnostic::new(ParseErrorCode::EofInTag, SourceLocation::new(2, 5, 9));
        assert_eq!(diagnostic.to_string(), "eof-in-tag at 2:5:offset 9");
        assert_eq!(diagnostic.code().as_str(), "eof-in-tag");
        assert_eq!(diagnostic.location().line(), 2);
    }

    #[test]
    fn diagnostics_keep_detection_order() {
        let mut diagnostics = Diagnostics::new();
        assert!(diagnostics.is_empty());
        let at = SourceLocation::initial();
        diagnostics.push(ParseDiagnostic::new(ParseErrorCode::StrayEndTag, at));
        diagnostics.push(ParseDiagnostic::new(ParseErrorCode::EofInTag, at));
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics.first().map(ParseDiagnostic::code),
            Some(ParseErrorCode::StrayEndTag)
        );
        let codes: Vec<_> = diagnostics.iter().map(ParseDiagnostic::code).collect();
        assert_eq!(
            codes,
            [ParseErrorCode::StrayEndTag, ParseErrorCode::EofInTag]
        );
    }
}
