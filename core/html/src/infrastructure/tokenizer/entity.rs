//! HTML character reference resolution: WHATWG §13.2.5.72–§13.2.5.80.
//!
//! A reference that does not resolve is left as literal text. Parse errors are reported at the `&`
//! that opens the reference (ADR-0023); the spec reports some of them later, at the point of detection.

use crate::domain::diagnostic::ParseErrorCode;
use crate::domain::location::SourceLocation;
use crate::domain::named_reference::{NamedCharacterReference, ReferenceMatch};
use crate::domain::numeric_reference::{NumericReference, Radix};
use crate::infrastructure::tokenizer::cursor::Cursor;

/// Where a character reference appears: the spec treats the two differently (§13.2.5.73).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceContext {
    /// Character data.
    Text,
    /// An attribute value, quoted or not.
    AttributeValue,
}

/// Resolves the character reference that starts at the `&` the cursor just returned, appending its
/// text — or a literal `&` when it does not resolve — to `output`.
pub fn consume_character_reference(
    cursor: &mut Cursor<'_>,
    output: &mut String,
    context: ReferenceContext,
) {
    let ampersand = cursor.last_location();
    match cursor.peek() {
        Some('#') => consume_numeric_reference(cursor, output, ampersand),
        Some(next) if next.is_ascii_alphanumeric() => {
            consume_named_reference(cursor, output, ampersand, context);
        }
        _ => output.push('&'),
    }
}

fn consume_named_reference(
    cursor: &mut Cursor<'_>,
    output: &mut String,
    ampersand: SourceLocation,
    context: ReferenceContext,
) {
    let Some(found) = NamedCharacterReference::longest_match(cursor.remaining()) else {
        report_unknown_name(cursor, ampersand);
        output.push('&');
        return;
    };
    if is_left_literal(&found, cursor.remaining(), context) {
        output.push('&');
        return;
    }
    advance(cursor, found.consumed());
    if !found.is_terminated() {
        cursor.report(
            ParseErrorCode::MissingSemicolonAfterCharacterReference,
            ampersand,
        );
    }
    output.push_str(found.reference().expansion());
}

/// §13.2.5.73: in an attribute value, a name without `;` followed by `=` or a letter or digit is
/// historical text (`?a=1&copy=2`), not a reference — and not an error.
fn is_left_literal(found: &ReferenceMatch, remaining: &str, context: ReferenceContext) -> bool {
    if context != ReferenceContext::AttributeValue || found.is_terminated() {
        return false;
    }
    let following = remaining
        .get(found.consumed()..)
        .and_then(|rest| rest.chars().next());
    following.is_some_and(|next| next == '=' || next.is_ascii_alphanumeric())
}

/// §13.2.5.74: a run of letters and digits closed by `;` names nothing.
fn report_unknown_name(cursor: &mut Cursor<'_>, ampersand: SourceLocation) {
    let name_length = cursor
        .remaining()
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .count();
    if cursor.remaining().chars().nth(name_length) == Some(';') {
        cursor.report(ParseErrorCode::UnknownNamedCharacterReference, ampersand);
    }
}

fn consume_numeric_reference(
    cursor: &mut Cursor<'_>,
    output: &mut String,
    ampersand: SourceLocation,
) {
    let radix = Radix::of_reference(cursor.remaining());
    let digits: String = cursor
        .remaining()
        .chars()
        .skip(radix.prefix_length())
        .take_while(|digit| digit.is_digit(radix.base()))
        .collect();
    if digits.is_empty() {
        cursor.report(
            ParseErrorCode::AbsenceOfDigitsInNumericCharacterReference,
            ampersand,
        );
        output.push('&');
        return;
    }
    advance(cursor, radix.prefix_length().saturating_add(digits.len()));
    consume_numeric_terminator(cursor, ampersand);
    let resolved = NumericReference::parse(&digits, radix).resolve();
    if let Some(error) = resolved.error() {
        cursor.report(error, ampersand);
    }
    output.push(resolved.character());
}

fn consume_numeric_terminator(cursor: &mut Cursor<'_>, ampersand: SourceLocation) {
    if cursor.peek() == Some(';') {
        cursor.next_char();
        return;
    }
    cursor.report(
        ParseErrorCode::MissingSemicolonAfterCharacterReference,
        ampersand,
    );
}

fn advance(cursor: &mut Cursor<'_>, characters: usize) {
    for _ in 0..characters {
        cursor.next_char();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::diagnostic::ParseDiagnostic;

    fn consume(source: &str, context: ReferenceContext) -> (String, Vec<ParseErrorCode>, String) {
        let mut cursor = Cursor::new(source);
        cursor.next_char();
        let mut output = String::new();
        consume_character_reference(&mut cursor, &mut output, context);
        let codes = cursor
            .take_diagnostics()
            .iter()
            .map(ParseDiagnostic::code)
            .collect();
        (output, codes, cursor.remaining().to_string())
    }

    fn in_text(source: &str) -> (String, Vec<ParseErrorCode>, String) {
        consume(source, ReferenceContext::Text)
    }

    fn reported(source: &str) -> Vec<ParseErrorCode> {
        in_text(source).1
    }

    #[test]
    fn named_references_resolve_from_the_full_table() {
        assert_eq!(in_text("&amp;").0, "&");
        assert_eq!(in_text("&hellip;").0, "…");
        assert_eq!(in_text("&euro;").0, "€");
        assert_eq!(in_text("&NotEqualTilde;").0, "\u{2242}\u{338}");
    }

    #[test]
    fn a_legacy_name_resolves_without_its_semicolon_and_is_reported() {
        assert_eq!(
            in_text("&copy rest"),
            (
                "©".into(),
                vec![ParseErrorCode::MissingSemicolonAfterCharacterReference],
                " rest".into()
            )
        );
    }

    #[test]
    fn the_longest_legacy_prefix_wins_when_the_full_name_is_unknown() {
        assert_eq!(
            in_text("&noti;"),
            (
                "¬".into(),
                vec![ParseErrorCode::MissingSemicolonAfterCharacterReference],
                "i;".into()
            )
        );
    }

    #[test]
    fn a_name_that_needs_its_semicolon_stays_text() {
        assert_eq!(
            in_text("&hellip b"),
            ("&".into(), vec![], "hellip b".into())
        );
    }

    #[test]
    fn in_an_attribute_value_a_legacy_name_before_equals_or_alphanumeric_is_literal() {
        let literal = ("&".into(), vec![], "not=".into());
        assert_eq!(consume("&not=", ReferenceContext::AttributeValue), literal);
        let literal = ("&".into(), vec![], "notx".into());
        assert_eq!(consume("&notx", ReferenceContext::AttributeValue), literal);
    }

    #[test]
    fn in_an_attribute_value_a_legacy_name_before_anything_else_still_resolves() {
        let (output, codes, _) = consume("&not ", ReferenceContext::AttributeValue);
        assert_eq!(output, "¬");
        assert_eq!(
            codes,
            [ParseErrorCode::MissingSemicolonAfterCharacterReference]
        );
    }

    #[test]
    fn unresolved_references_are_classified() {
        assert_eq!(
            reported("&bogus;"),
            [ParseErrorCode::UnknownNamedCharacterReference]
        );
        assert_eq!(
            reported("&#;"),
            [ParseErrorCode::AbsenceOfDigitsInNumericCharacterReference]
        );
        assert_eq!(
            reported("&#x;"),
            [ParseErrorCode::AbsenceOfDigitsInNumericCharacterReference]
        );
        assert_eq!(reported("&#0;"), [ParseErrorCode::NullCharacterReference]);
        assert_eq!(
            reported("&#xD800;"),
            [ParseErrorCode::SurrogateCharacterReference]
        );
        assert_eq!(
            reported("&#x110000;"),
            [ParseErrorCode::CharacterReferenceOutsideUnicodeRange]
        );
        assert_eq!(
            reported("&#99999999999;"),
            [ParseErrorCode::CharacterReferenceOutsideUnicodeRange]
        );
    }

    #[test]
    fn an_invalid_code_point_reference_becomes_the_replacement_character() {
        let (output, _, remaining) = in_text("&#xD800;z");
        assert_eq!(output, "\u{FFFD}");
        assert_eq!(remaining, "z");
    }

    #[test]
    fn an_unterminated_numeric_reference_still_resolves_and_is_reported() {
        assert_eq!(
            in_text("&#97ab"),
            (
                "a".into(),
                vec![ParseErrorCode::MissingSemicolonAfterCharacterReference],
                "ab".into()
            )
        );
    }

    #[test]
    fn numeric_references_follow_the_windows_1252_remap() {
        assert_eq!(
            in_text("&#128;"),
            (
                "€".into(),
                vec![ParseErrorCode::ControlCharacterReference],
                String::new()
            )
        );
        assert_eq!(
            reported("&#xFDD0;"),
            [ParseErrorCode::NoncharacterCharacterReference]
        );
    }

    #[test]
    fn plain_ampersands_and_resolved_references_are_silent() {
        assert!(reported("& b").is_empty());
        assert!(reported("&amp;").is_empty());
        assert!(reported("&#60;").is_empty());
        assert!(reported("&bogus b").is_empty());
        assert!(reported("&").is_empty());
    }

    #[test]
    fn numeric_references_in_both_radixes_resolve() {
        assert_eq!(in_text("&#60;").0, "<");
        assert_eq!(in_text("&#x3e;").0, ">");
        assert_eq!(in_text("&#X3E;").0, ">");
    }
}
