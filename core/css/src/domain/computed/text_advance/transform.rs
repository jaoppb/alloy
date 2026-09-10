//! [`TextTransform`] — text casing transformation (CSS Text L3 §3.1).

use core::fmt;

/// Casing transformations applied to element text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextTransform {
    /// No casing transformation — CSS `initial`.
    #[default]
    None,
    /// Capitalize first character of each word.
    Capitalize,
    /// Convert all characters to uppercase.
    Uppercase,
    /// Convert all characters to lowercase.
    Lowercase,
}

impl TextTransform {
    /// The keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Capitalize => "capitalize",
            Self::Uppercase => "uppercase",
            Self::Lowercase => "lowercase",
        }
    }

    /// Transforms a string slice according to this text transformation rule.
    #[must_use]
    pub fn apply(self, text: &str) -> String {
        match self {
            Self::None => text.to_string(),
            Self::Uppercase => text.to_uppercase(),
            Self::Lowercase => text.to_lowercase(),
            Self::Capitalize => capitalize_words(text),
        }
    }
}

fn append_uppercase(ch: char, result: &mut String) {
    for upper in ch.to_uppercase() {
        result.push(upper);
    }
}

fn step_capitalize_char(ch: char, capitalize_next: &mut bool, result: &mut String) {
    if ch.is_whitespace() {
        *capitalize_next = true;
        result.push(ch);
        return;
    }
    if *capitalize_next {
        append_uppercase(ch, result);
        *capitalize_next = false;
        return;
    }
    result.push(ch);
}

fn capitalize_words(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut capitalize_next = true;
    for ch in text.chars() {
        step_capitalize_char(ch, &mut capitalize_next, &mut result);
    }
    result
}

impl fmt::Display for TextTransform {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}
