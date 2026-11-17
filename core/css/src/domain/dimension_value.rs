//! [`DimensionValue`] — an HTML `width` / `height` attribute, read with the
//! "rules for parsing dimension values" (WHATWG HTML §2.3.4.4, issue #83).
//!
//! One parser serves both uses: the presentational hint the attribute maps to
//! (HTML §15.4.3) and the preferred ratio a replaced element takes from it
//! before its resource loads. [`non_negative_integer`] reads the integer
//! attributes beside them — a `<canvas>`'s bitmap size.

use core::fmt::Write;

use graphics::{AU_PER_PX, Au};

/// The most fractional digits kept: more cannot change an [`Au`] (1/64 px).
const MAX_FRACTION_DIGITS: usize = 9;

/// A parsed dimension attribute: a non-negative number of CSS pixels, or a
/// percentage. Whatever follows the number (`px`, junk) is ignored, as the
/// spec's rules do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DimensionValue {
    whole: String,
    fraction: String,
    unit: DimensionUnit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DimensionUnit {
    Pixels,
    Percent,
}

impl DimensionValue {
    /// Leading white space, digits, an optional `.` and fraction digits, then
    /// `%` for a percentage; `None` when no digit starts the value.
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        let rest = raw.trim_start_matches(|character: char| character.is_ascii_whitespace());
        let whole = leading_digits(rest);
        if whole.is_empty() {
            return None;
        }
        let after_whole = rest.get(whole.len()..)?;
        let (fraction, unit) = parse_fraction_and_unit(after_whole);
        Some(Self {
            whole: whole.to_owned(),
            fraction: fraction.chars().take(MAX_FRACTION_DIGITS).collect(),
            unit,
        })
    }

    /// The CSS value the attribute maps to: `100px`, `12.5px`, `50%`.
    pub(crate) fn to_css(&self) -> String {
        let mut css = self.whole.clone();
        if !self.fraction.is_empty() {
            let _ = write!(css, ".{}", self.fraction);
        }
        css.push_str(self.unit.suffix());
        css
    }

    /// The value in [`Au`] when it is a length — a percentage has no length of
    /// its own — or `None` when it is too large to lay out.
    pub(crate) fn pixels(&self) -> Option<Au> {
        if self.unit == DimensionUnit::Percent {
            return None;
        }
        let whole: i64 = self.whole.parse().ok()?;
        let raw = whole
            .checked_mul(i64::from(AU_PER_PX))?
            .checked_add(fraction_raw(&self.fraction)?)?;
        Some(Au::from_raw(i32::try_from(raw).ok()?))
    }
}

impl DimensionUnit {
    const fn suffix(self) -> &'static str {
        match self {
            Self::Pixels => "px",
            Self::Percent => "%",
        }
    }
}

/// An attribute read with the rules for parsing non-negative integers (HTML
/// §2.3.4.2, through §2.3.4.1): leading white space, an optional `+`, digits;
/// a `-` passes only when the digits are all zero. Whatever follows is
/// ignored.
pub(crate) fn non_negative_integer(raw: &str) -> Option<u32> {
    let rest = raw.trim_start_matches(|character: char| character.is_ascii_whitespace());
    if let Some(negative) = rest.strip_prefix('-') {
        return leading_digits(negative)
            .parse::<u32>()
            .ok()
            .filter(|value| *value == 0);
    }
    let unsigned = rest.strip_prefix('+').unwrap_or(rest);
    leading_digits(unsigned).parse().ok()
}

fn leading_digits(text: &str) -> &str {
    let end = text
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(text.len());
    text.get(..end).unwrap_or("")
}

/// HTML §2.3.4.4 step 9.2: a `.` without a following digit returns a length
/// immediately, before checking for `%`.
fn parse_fraction_and_unit(after_whole: &str) -> (&str, DimensionUnit) {
    after_whole
        .strip_prefix('.')
        .map_or_else(|| ("", unit_of(after_whole)), fraction_after_dot)
}

fn fraction_after_dot(after_dot: &str) -> (&str, DimensionUnit) {
    let digits = leading_digits(after_dot);
    if digits.is_empty() {
        return ("", DimensionUnit::Pixels);
    }
    let after_fraction = after_dot.get(digits.len()..).unwrap_or("");
    (digits, unit_of(after_fraction))
}

fn unit_of(after_number: &str) -> DimensionUnit {
    if after_number.starts_with('%') {
        return DimensionUnit::Percent;
    }
    DimensionUnit::Pixels
}

/// The fraction digits as `Au` units, rounded down.
fn fraction_raw(fraction: &str) -> Option<i64> {
    if fraction.is_empty() {
        return Some(0);
    }
    let digits: i64 = fraction.parse().ok()?;
    let scale = 10_i64.checked_pow(u32::try_from(fraction.len()).ok()?)?;
    digits.checked_mul(i64::from(AU_PER_PX))?.checked_div(scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn css(raw: &str) -> Option<String> {
        DimensionValue::parse(raw).map(|value| value.to_css())
    }

    fn pixels(raw: &str) -> Option<Au> {
        DimensionValue::parse(raw).and_then(|value| value.pixels())
    }

    #[test]
    fn a_number_is_pixels_whatever_follows_it() {
        assert_eq!(css(" 100"), Some("100px".to_owned()));
        assert_eq!(css("100px"), Some("100px".to_owned()));
        assert_eq!(css("100abc"), Some("100px".to_owned()));
        assert_eq!(css("12.5"), Some("12.5px".to_owned()));
        assert_eq!(css("12."), Some("12px".to_owned()));
        assert_eq!(css("12.%"), Some("12px".to_owned()));
        assert_eq!(css("0"), Some("0px".to_owned()));
        assert_eq!(css("0px"), Some("0px".to_owned()));
    }

    #[test]
    fn a_trailing_percent_sign_makes_a_percentage() {
        assert_eq!(css("50%"), Some("50%".to_owned()));
        assert_eq!(css("0%"), Some("0%".to_owned()));
        assert_eq!(pixels("50%"), None);
    }

    #[test]
    fn a_value_that_starts_without_a_digit_is_ignored() {
        assert_eq!(css(""), None);
        assert_eq!(css("auto"), None);
        assert_eq!(css("-5"), None);
        assert_eq!(css("+100"), None);
        assert_eq!(css(".5"), None);
        assert_eq!(css("\u{00A0}100"), None);
    }

    #[test]
    fn a_non_negative_integer_takes_a_plus_sign_and_minus_zero() {
        assert_eq!(non_negative_integer(" 400px"), Some(400));
        assert_eq!(non_negative_integer("+400"), Some(400));
        assert_eq!(non_negative_integer("-0"), Some(0));
        assert_eq!(non_negative_integer("-0abc"), Some(0));
        assert_eq!(non_negative_integer("-5"), None);
        assert_eq!(non_negative_integer("auto"), None);
        assert_eq!(non_negative_integer("4294967296"), None);
    }

    #[test]
    fn pixels_keep_their_fraction_in_au() {
        assert_eq!(pixels("100px"), Au::from_whole_px(100));
        assert_eq!(pixels("0.5"), Some(Au::from_raw(AU_PER_PX / 2)));
        assert_eq!(pixels("99999999999"), None);
    }
}
