//! [`NamedCharacterReference`] — a name from the WHATWG named character reference table (§13.5).
//!
//! One table serves both directions: the tokenizer decodes with [`NamedCharacterReference::longest_match`]
//! and the `dom` serializer encodes the five-name escape set (§13.3) through
//! [`NamedCharacterReference::from_char`]. The expansion is text, not a `char`, because 93 references
//! expand to two code points (`&NotEqualTilde;` → U+2242 U+0338).

use super::named_references::{ROWS, ReferenceRow};
use core::cmp::Ordering;
use core::fmt;

/// Length of the longest name in the table (`CounterClockwiseContourIntegral`), without `&` or `;`.
const LONGEST_NAME_LENGTH: usize = 31;

/// A named character reference: a handle on one row of the WHATWG table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NamedCharacterReference(&'static ReferenceRow);

/// How a run of input text matched the table (§13.2.5.73).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferenceMatch {
    reference: NamedCharacterReference,
    consumed: usize,
    terminated: bool,
}

impl ReferenceMatch {
    /// The reference that matched.
    #[must_use]
    pub const fn reference(&self) -> NamedCharacterReference {
        self.reference
    }

    /// Characters the match covers, trailing `;` included.
    #[must_use]
    pub const fn consumed(&self) -> usize {
        self.consumed
    }

    /// Whether the match ends in `;` (otherwise it is a legacy semicolon-less name).
    #[must_use]
    pub const fn is_terminated(&self) -> bool {
        self.terminated
    }
}

impl NamedCharacterReference {
    /// `&amp;` — the escape set of WHATWG §13.3, resolved at compile time: a name missing from the
    /// table is a build error, not a test failure.
    pub const AMP: Self = Self::resolved("amp");
    /// `&lt;`.
    pub const LT: Self = Self::resolved("lt");
    /// `&gt;`.
    pub const GT: Self = Self::resolved("gt");
    /// `&quot;`.
    pub const QUOT: Self = Self::resolved("quot");
    /// `&nbsp;`.
    pub const NBSP: Self = Self::resolved("nbsp");

    const ESCAPED: [Self; 5] = [Self::AMP, Self::LT, Self::GT, Self::QUOT, Self::NBSP];

    #[allow(
        clippy::panic,
        reason = "evaluated only in a const: a failure is a compile error"
    )]
    const fn resolved(name: &str) -> Self {
        match row_named(name) {
            Some(row) => Self(row),
            None => panic!("escape-set name missing from the WHATWG table"),
        }
    }

    /// Looks a reference up by its name without delimiters (`"copy"` for `&copy;`).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        row_named(name).map(Self)
    }

    /// The escape-set reference for `character`, if WHATWG §13.3 escapes it when serializing.
    #[must_use]
    pub fn from_char(character: char) -> Option<Self> {
        let mut buffer = [0; 4];
        let encoded: &str = character.encode_utf8(&mut buffer);
        Self::ESCAPED
            .into_iter()
            .find(|reference| reference.expansion() == encoded)
    }

    /// The longest reference `text` starts with, matching names with `;` and legacy names without.
    #[must_use]
    pub fn longest_match(text: &str) -> Option<ReferenceMatch> {
        (1..=LONGEST_NAME_LENGTH.min(text.len()))
            .rev()
            .find_map(|length| Self::match_name_of_length(text, length))
    }

    fn match_name_of_length(text: &str, length: usize) -> Option<ReferenceMatch> {
        let reference = Self::from_name(text.get(..length)?)?;
        let terminated = text.get(length..)?.starts_with(';');
        if !terminated && !reference.is_semicolon_optional() {
            return None;
        }
        let consumed = if terminated {
            length.saturating_add(1)
        } else {
            length
        };
        Some(ReferenceMatch {
            reference,
            consumed,
            terminated,
        })
    }

    /// The text this reference expands to: one or two code points.
    #[must_use]
    pub const fn expansion(&self) -> &'static str {
        self.0.expansion
    }

    /// The name without delimiters (`"copy"` for `&copy;`).
    #[must_use]
    pub const fn entity_name(&self) -> &'static str {
        self.0.name
    }

    /// Whether the spec also accepts this name without its trailing `;` (the 106 legacy names).
    #[must_use]
    pub const fn is_semicolon_optional(&self) -> bool {
        self.0.semicolon_optional
    }

    /// Appends the serialized form (`&copy;`) to `output`.
    pub fn append_entity(&self, output: &mut String) {
        output.push('&');
        output.push_str(self.entity_name());
        output.push(';');
    }
}

impl fmt::Display for NamedCharacterReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "&{};", self.entity_name())
    }
}

const fn row_named(name: &str) -> Option<&'static ReferenceRow> {
    let mut low = 0;
    let mut high = ROWS.len();
    while low < high {
        let middle = low.saturating_add(high.saturating_sub(low) / 2);
        let Some(row) = row_at(middle) else {
            return None;
        };
        match compare_bytes(name.as_bytes(), row.name.as_bytes()) {
            Ordering::Equal => return Some(row),
            Ordering::Less => high = middle,
            Ordering::Greater => low = middle.saturating_add(1),
        }
    }
    None
}

/// `ROWS[index]` without indexing: `slice::get` is not yet a `const fn`.
const fn row_at(index: usize) -> Option<&'static ReferenceRow> {
    match ROWS.split_at_checked(index) {
        Some((_, rest)) => rest.first(),
        None => None,
    }
}

/// Bytewise comparison usable in `const` (`<[u8]>::cmp` is not).
const fn compare_bytes(left: &[u8], right: &[u8]) -> Ordering {
    let (mut left, mut right) = (left, right);
    loop {
        match (left.split_first(), right.split_first()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some((left_byte, left_rest)), Some((right_byte, right_rest))) => {
                if *left_byte < *right_byte {
                    return Ordering::Less;
                }
                if *left_byte > *right_byte {
                    return Ordering::Greater;
                }
                left = left_rest;
                right = right_rest;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_sorted_and_free_of_duplicates() {
        assert!(ROWS.windows(2).all(|pair| pair[0].name < pair[1].name));
    }

    #[test]
    fn the_longest_name_length_constant_matches_the_table() {
        let longest = ROWS.iter().map(|row| row.name.len()).max();
        assert_eq!(longest, Some(LONGEST_NAME_LENGTH));
    }

    #[test]
    fn every_row_is_found_by_its_own_name() {
        for row in ROWS {
            assert_eq!(
                NamedCharacterReference::from_name(row.name).map(|r| r.0),
                Some(row)
            );
        }
    }

    #[test]
    fn lookups_by_name() {
        let hellip = NamedCharacterReference::from_name("hellip").unwrap();
        assert_eq!(hellip.expansion(), "\u{2026}");
        assert_eq!(hellip.to_string(), "&hellip;");
        assert_eq!(NamedCharacterReference::from_name("unknown"), None);
        assert_eq!(NamedCharacterReference::from_name(""), None);
        assert_eq!(NamedCharacterReference::from_name("amp;"), None);
    }

    #[test]
    fn a_reference_can_expand_to_two_code_points() {
        let reference = NamedCharacterReference::from_name("NotEqualTilde").unwrap();
        assert_eq!(reference.expansion(), "\u{2242}\u{338}");
    }

    #[test]
    fn only_the_escape_set_is_found_by_character() {
        assert_eq!(
            NamedCharacterReference::from_char('&'),
            Some(NamedCharacterReference::AMP)
        );
        assert_eq!(
            NamedCharacterReference::from_char('<'),
            Some(NamedCharacterReference::LT)
        );
        assert_eq!(
            NamedCharacterReference::from_char('>'),
            Some(NamedCharacterReference::GT)
        );
        assert_eq!(
            NamedCharacterReference::from_char('"'),
            Some(NamedCharacterReference::QUOT)
        );
        assert_eq!(
            NamedCharacterReference::from_char('\u{a0}'),
            Some(NamedCharacterReference::NBSP)
        );
        assert_eq!(NamedCharacterReference::from_char('\u{a9}'), None);
        assert_eq!(NamedCharacterReference::from_char('\u{e9}'), None);
    }

    #[test]
    fn append_entity_serializes_with_delimiters() {
        let mut output = String::from("x");
        NamedCharacterReference::AMP.append_entity(&mut output);
        assert_eq!(output, "x&amp;");
    }

    #[test]
    fn a_semicolon_terminated_name_matches_with_its_semicolon() {
        let found = NamedCharacterReference::longest_match("copy; rest").unwrap();
        assert_eq!(found.reference().entity_name(), "copy");
        assert_eq!((found.consumed(), found.is_terminated()), (5, true));
    }

    #[test]
    fn a_legacy_name_matches_without_a_semicolon() {
        let found = NamedCharacterReference::longest_match("copy rest").unwrap();
        assert_eq!((found.consumed(), found.is_terminated()), (4, false));
    }

    #[test]
    fn the_longest_prefix_wins_and_falls_back_to_a_legacy_prefix() {
        let longer = NamedCharacterReference::longest_match("notin;").unwrap();
        assert_eq!(longer.reference().entity_name(), "notin");
        let fallback = NamedCharacterReference::longest_match("notit;").unwrap();
        assert_eq!(fallback.reference().entity_name(), "not");
        assert_eq!((fallback.consumed(), fallback.is_terminated()), (3, false));
    }

    #[test]
    fn a_non_legacy_name_needs_its_semicolon() {
        assert_eq!(NamedCharacterReference::longest_match("hellip rest"), None);
        assert_eq!(NamedCharacterReference::longest_match("& b"), None);
        assert_eq!(NamedCharacterReference::longest_match(""), None);
    }

    #[test]
    fn non_ascii_text_never_matches_or_panics() {
        assert_eq!(NamedCharacterReference::longest_match("é;"), None);
        assert_eq!(NamedCharacterReference::longest_match("coé;"), None);
    }
}
