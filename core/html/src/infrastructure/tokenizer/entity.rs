//! HTML entity resolution for named, decimal, and hexadecimal character references.

use crate::domain::diagnostic::ParseErrorCode;
use crate::domain::named_reference::NamedCharacterReference;
use crate::infrastructure::tokenizer::cursor::Cursor;

/// Resolves an HTML character reference from the cursor stream.
///
/// A reference that does not resolve is left as literal text (`&`), and — where WHATWG §13.2.5.72+
/// names the malformation — reported at the `&`.
pub fn consume_character_reference(cursor: &mut Cursor<'_>, output: &mut String) {
    let ampersand = cursor.last_location();
    let mut candidate = String::new();
    let mut cloned_cursor = cursor.clone();
    let mut matched_chars = 0_usize;
    let mut terminated = false;

    while let Some(character) = cloned_cursor.next_char() {
        matched_chars = matched_chars.saturating_add(1);
        if character == ';' {
            terminated = true;
            if let Some(resolved) = resolve_entity(&candidate) {
                output.push_str(&resolved);
                for _ in 0..matched_chars {
                    cursor.next_char();
                }
                return;
            }
            break;
        }
        if !character.is_ascii_alphanumeric() && character != '#' {
            break;
        }
        candidate.push(character);
        if candidate.len() > 16 {
            break;
        }
    }

    let Some(code) = unresolved_reference_error(&candidate, terminated) else {
        output.push('&');
        return;
    };
    cursor.report(code, ampersand);
    if terminated && is_invalid_code_point_error(code) {
        // WHATWG §13.2.5.80: a NUL, surrogate or out-of-range reference becomes U+FFFD.
        for _ in 0..matched_chars {
            cursor.next_char();
        }
        output.push(char::REPLACEMENT_CHARACTER);
        return;
    }
    output.push('&');
}

const fn is_invalid_code_point_error(code: ParseErrorCode) -> bool {
    matches!(
        code,
        ParseErrorCode::NullCharacterReference
            | ParseErrorCode::SurrogateCharacterReference
            | ParseErrorCode::CharacterReferenceOutsideUnicodeRange
    )
}

/// Which WHATWG error, if any, an unresolved `&candidate` (`;`-terminated or not) is.
fn unresolved_reference_error(candidate: &str, terminated: bool) -> Option<ParseErrorCode> {
    let Some(numeric) = candidate.strip_prefix('#') else {
        let is_named_reference = terminated && !candidate.is_empty();
        return is_named_reference.then_some(ParseErrorCode::UnknownNamedCharacterReference);
    };
    let (digits, radix) = numeric
        .strip_prefix(['x', 'X'])
        .map_or((numeric, 10), |hex| (hex, 16));
    if !digits
        .chars()
        .next()
        .is_some_and(|first| first.is_digit(radix))
    {
        return Some(ParseErrorCode::AbsenceOfDigitsInNumericCharacterReference);
    }
    if !digits.chars().all(|digit| digit.is_digit(radix)) {
        // Trailing letters after the digits (`&#12ab;`) are outside the codes this tokenizer models.
        return None;
    }
    let Ok(code_point) = u32::from_str_radix(digits, radix) else {
        return Some(ParseErrorCode::CharacterReferenceOutsideUnicodeRange);
    };
    match code_point {
        0 => Some(ParseErrorCode::NullCharacterReference),
        0xD800..=0xDFFF => Some(ParseErrorCode::SurrogateCharacterReference),
        0x11_0000.. => Some(ParseErrorCode::CharacterReferenceOutsideUnicodeRange),
        _ => None,
    }
}

/// Resolves a named or numeric entity name.
///
/// Named references cover only a small subset of the WHATWG table; the rest is tracked in
/// <https://github.com/jaoppb/alloy/issues/31>.
#[must_use]
pub fn resolve_entity(name: &str) -> Option<String> {
    if let Some(stripped) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
        let code = u32::from_str_radix(stripped, 16).ok()?;
        return char::from_u32(code)
            .filter(|resolved| *resolved != '\0')
            .map(String::from);
    }
    if let Some(stripped) = name.strip_prefix('#') {
        let code = stripped.parse::<u32>().ok()?;
        return char::from_u32(code)
            .filter(|resolved| *resolved != '\0')
            .map(String::from);
    }

    NamedCharacterReference::from_name(name).map(|entity| entity.expansion().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::diagnostic::ParseDiagnostic;

    #[test]
    fn resolve_named_entities() {
        assert_eq!(resolve_entity("amp").as_deref(), Some("&"));
        assert_eq!(resolve_entity("copy").as_deref(), Some("©"));
        assert_eq!(resolve_entity("hellip").as_deref(), Some("…"));
        assert_eq!(resolve_entity("unknown"), None);
    }

    fn reported(source: &str) -> Vec<ParseErrorCode> {
        let mut cursor = Cursor::new(source);
        cursor.next_char();
        let mut output = String::new();
        consume_character_reference(&mut cursor, &mut output);
        cursor
            .take_diagnostics()
            .iter()
            .map(ParseDiagnostic::code)
            .collect()
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
        let mut cursor = Cursor::new("&#xD800;z");
        cursor.next_char();
        let mut output = String::new();
        consume_character_reference(&mut cursor, &mut output);
        assert_eq!(output, "\u{FFFD}");
        assert_eq!(cursor.next_char(), Some('z'));
    }

    #[test]
    fn plain_ampersands_and_resolved_references_are_silent() {
        assert!(reported("& b").is_empty());
        assert!(reported("&amp;").is_empty());
        assert!(reported("&#60;").is_empty());
        assert!(reported("&bogus b").is_empty());
    }

    #[test]
    fn resolve_numeric_entities() {
        assert_eq!(resolve_entity("#60").as_deref(), Some("<"));
        assert_eq!(resolve_entity("#x3e").as_deref(), Some(">"));
        assert_eq!(resolve_entity("#X3E").as_deref(), Some(">"));
    }
}
