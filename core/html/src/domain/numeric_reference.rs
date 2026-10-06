//! [`NumericReference`] — the code point a `&#…;` reference names, and what WHATWG §13.2.5.80 makes of it.

use super::diagnostic::ParseErrorCode;

/// U+FFFD, what an invalid code point becomes.
const REPLACEMENT: char = char::REPLACEMENT_CHARACTER;

/// The Windows-1252 characters §13.2.5.80 substitutes for the C1 controls U+0080..=U+009F.
const WINDOWS_1252: [char; 32] = [
    '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}', '\u{017D}', '\u{008F}',
    '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
];

/// The base a numeric reference is written in: `&#97;` or `&#x61;`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Radix {
    /// `&#…;`.
    Decimal,
    /// `&#x…;` / `&#X…;`.
    Hexadecimal,
}

impl Radix {
    /// The radix a reference opening with `text` (which starts at the `#`) is written in.
    #[must_use]
    pub fn of_reference(text: &str) -> Self {
        let marker = text.chars().nth(1);
        match marker {
            Some('x' | 'X') => Self::Hexadecimal,
            _ => Self::Decimal,
        }
    }

    /// The number base.
    #[must_use]
    pub const fn base(self) -> u32 {
        match self {
            Self::Decimal => 10,
            Self::Hexadecimal => 16,
        }
    }

    /// Characters before the digits: `#` or `#x`.
    #[must_use]
    pub const fn prefix_length(self) -> usize {
        match self {
            Self::Decimal => 1,
            Self::Hexadecimal => 2,
        }
    }
}

/// The number a numeric reference spells, before §13.2.5.80 validates it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumericReference(u32);

/// The character a numeric reference becomes, and the error (if any) the spec reports for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedReference {
    character: char,
    error: Option<ParseErrorCode>,
}

impl ResolvedReference {
    /// The character to emit.
    #[must_use]
    pub const fn character(&self) -> char {
        self.character
    }

    /// The parse error to report, if the reference was malformed.
    #[must_use]
    pub const fn error(&self) -> Option<ParseErrorCode> {
        self.error
    }
}

impl NumericReference {
    /// Reads `digits` (all valid in `radix`); a number too large for `u32` saturates, which is far
    /// beyond U+10FFFF and so resolves the same way.
    #[must_use]
    pub fn parse(digits: &str, radix: Radix) -> Self {
        let value = digits
            .chars()
            .filter_map(|digit| digit.to_digit(radix.base()))
            .fold(0_u32, |accumulated, digit| {
                accumulated
                    .saturating_mul(radix.base())
                    .saturating_add(digit)
            });
        Self(value)
    }

    /// Applies §13.2.5.80: invalid code points become U+FFFD, the C1 controls are remapped.
    #[must_use]
    pub fn resolve(self) -> ResolvedReference {
        if let Some(error) = invalid_code_point_error(self.0) {
            return ResolvedReference {
                character: REPLACEMENT,
                error: Some(error),
            };
        }
        let character = char::from_u32(self.0).unwrap_or(REPLACEMENT);
        ResolvedReference {
            character: windows_1252(self.0).unwrap_or(character),
            error: questionable_code_point_error(self.0),
        }
    }
}

const fn invalid_code_point_error(value: u32) -> Option<ParseErrorCode> {
    match value {
        0 => Some(ParseErrorCode::NullCharacterReference),
        0x11_0000.. => Some(ParseErrorCode::CharacterReferenceOutsideUnicodeRange),
        0xD800..=0xDFFF => Some(ParseErrorCode::SurrogateCharacterReference),
        _ => None,
    }
}

/// A valid code point the spec still reports: a noncharacter or a control.
fn questionable_code_point_error(value: u32) -> Option<ParseErrorCode> {
    if is_noncharacter(value) {
        return Some(ParseErrorCode::NoncharacterCharacterReference);
    }
    is_reported_control(value).then_some(ParseErrorCode::ControlCharacterReference)
}

fn is_noncharacter(value: u32) -> bool {
    let is_in_arabic_presentation_block = (0xFDD0..=0xFDEF).contains(&value);
    is_in_arabic_presentation_block || matches!(value & 0xFFFF, 0xFFFE | 0xFFFF)
}

/// U+000D, or a control character that is not ASCII whitespace.
const fn is_reported_control(value: u32) -> bool {
    let Some(character) = char::from_u32(value) else {
        return false;
    };
    character == '\r' || (character.is_control() && !character.is_ascii_whitespace())
}

fn windows_1252(value: u32) -> Option<char> {
    let offset = value.checked_sub(0x80)?;
    let index = usize::try_from(offset).ok()?;
    WINDOWS_1252.get(index).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(digits: &str, radix: Radix) -> (char, Option<ParseErrorCode>) {
        let reference = NumericReference::parse(digits, radix).resolve();
        (reference.character(), reference.error())
    }

    #[test]
    fn an_ordinary_code_point_resolves_silently() {
        assert_eq!(resolved("97", Radix::Decimal), ('a', None));
        assert_eq!(resolved("3e", Radix::Hexadecimal), ('>', None));
        assert_eq!(resolved("9", Radix::Decimal), ('\t', None));
    }

    #[test]
    fn invalid_code_points_become_the_replacement_character() {
        let null = (REPLACEMENT, Some(ParseErrorCode::NullCharacterReference));
        assert_eq!(resolved("0", Radix::Decimal), null);
        let surrogate = (
            REPLACEMENT,
            Some(ParseErrorCode::SurrogateCharacterReference),
        );
        assert_eq!(resolved("D800", Radix::Hexadecimal), surrogate);
        let outside = (
            REPLACEMENT,
            Some(ParseErrorCode::CharacterReferenceOutsideUnicodeRange),
        );
        assert_eq!(resolved("110000", Radix::Hexadecimal), outside);
        assert_eq!(resolved("99999999999999999999", Radix::Decimal), outside);
    }

    #[test]
    fn c1_controls_are_remapped_and_reported() {
        let control = Some(ParseErrorCode::ControlCharacterReference);
        assert_eq!(resolved("128", Radix::Decimal), ('\u{20AC}', control));
        assert_eq!(resolved("81", Radix::Hexadecimal), ('\u{0081}', control));
        assert_eq!(resolved("9f", Radix::Hexadecimal), ('\u{0178}', control));
    }

    #[test]
    fn carriage_return_and_other_controls_are_reported_but_whitespace_is_not() {
        let control = Some(ParseErrorCode::ControlCharacterReference);
        assert_eq!(resolved("13", Radix::Decimal), ('\r', control));
        assert_eq!(resolved("1", Radix::Decimal), ('\u{1}', control));
        assert_eq!(resolved("7f", Radix::Hexadecimal), ('\u{7f}', control));
        assert_eq!(resolved("12", Radix::Decimal), ('\u{c}', None));
    }

    #[test]
    fn noncharacters_are_reported_and_kept() {
        let error = Some(ParseErrorCode::NoncharacterCharacterReference);
        assert_eq!(resolved("fdd0", Radix::Hexadecimal), ('\u{FDD0}', error));
        assert_eq!(resolved("1FFFF", Radix::Hexadecimal), ('\u{1FFFF}', error));
    }

    #[test]
    fn the_radix_is_read_from_the_reference_opening() {
        assert_eq!(Radix::of_reference("#x61;"), Radix::Hexadecimal);
        assert_eq!(Radix::of_reference("#X61;"), Radix::Hexadecimal);
        assert_eq!(Radix::of_reference("#61;"), Radix::Decimal);
        assert_eq!(Radix::Hexadecimal.prefix_length(), 2);
    }
}
