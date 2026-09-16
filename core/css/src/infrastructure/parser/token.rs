//! The CSS Syntax Level 3 token vocabulary and the cursor over it.
//!
//! [`Token`] is deliberately close to the spec's §4 token list: keeping the
//! same names is what lets the recovery rules of §5.4 be read off the standard
//! instead of reinvented. The payloads are plain `String`s rather than
//! [`crate::Identifier`]s because tokenizing is **total** — it never fails — and
//! a name that no identifier can hold has to survive as far as the rule parser,
//! where refusing it produces a `CssError` with a span.

use core::fmt;

use crate::domain::error::SourceSpan;

/// One CSS token.
///
/// `#[non_exhaustive]`: the unicode-range and `<!--` / `-->` tokens of the full
/// grammar are not produced today and would arrive here.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Token {
    /// An identifier — `div`, `red`, `margin-top`.
    Ident(String),
    /// `@media`, `@import` — the name without the `@`.
    AtKeyword(String),
    /// `#main`, `#0af` — the text after the `#`.
    Hash(String),
    /// A quoted string, with quotes removed and escapes resolved.
    QuotedString(String),
    /// A string that hit a newline or the end of the source before its closing
    /// quote (CSS Syntax L3 §4.3.5).
    BadString,
    /// A name immediately followed by `(` — `rgb(`, `nth-child(`.
    Function(String),
    /// An unquoted `url(…)` payload.
    Url(String),
    /// A `url(` whose payload could not be read.
    BadUrl,
    /// A bare number — `0`, `1.5`, `-3e2`.
    Number(f32),
    /// A number with a unit — `16px`, `1.2em`.
    Dimension(f32, String),
    /// A number followed by `%`.
    Percentage(f32),
    /// Any other single character — `>`, `+`, `~`, `*`, `.`, `=`, `!`, `|`.
    Delimiter(char),
    /// One run of whitespace, however long.
    Whitespace,
    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `,`
    Comma,
    /// `{`
    OpenBrace,
    /// `}`
    CloseBrace,
    /// `(`
    OpenParenthesis,
    /// `)`
    CloseParenthesis,
    /// `[`
    OpenBracket,
    /// `]`
    CloseBracket,
}

impl Token {
    /// Whether this token is the whitespace run.
    #[must_use]
    pub const fn is_whitespace(&self) -> bool {
        matches!(self, Self::Whitespace)
    }

    /// The block opener this token closes, if it closes one — the pairing the
    /// recovery rules of CSS Syntax L3 §5.4 balance on.
    #[must_use]
    pub const fn closes(&self) -> Option<Self> {
        match self {
            Self::CloseBrace => Some(Self::OpenBrace),
            Self::CloseParenthesis => Some(Self::OpenParenthesis),
            Self::CloseBracket => Some(Self::OpenBracket),
            _ => None,
        }
    }

    /// The block closer this token opens, if it opens one.
    #[must_use]
    pub const fn opens(&self) -> Option<Self> {
        match self {
            Self::OpenBrace => Some(Self::CloseBrace),
            Self::OpenParenthesis => Some(Self::CloseParenthesis),
            Self::OpenBracket => Some(Self::CloseBracket),
            _ => None,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident(name) | Self::Url(name) => formatter.write_str(name),
            Self::AtKeyword(name) => write!(formatter, "@{name}"),
            Self::Hash(name) => write!(formatter, "#{name}"),
            Self::QuotedString(text) => write!(formatter, "\"{text}\""),
            Self::Function(name) => write!(formatter, "{name}("),
            Self::Number(value) => write!(formatter, "{value}"),
            Self::Dimension(value, unit) => write!(formatter, "{value}{unit}"),
            Self::Percentage(value) => write!(formatter, "{value}%"),
            Self::Delimiter(character) => write!(formatter, "{character}"),
            other => formatter.write_str(other.symbol()),
        }
    }
}

impl Token {
    /// The literal text of a token whose payload is fixed.
    const fn symbol(&self) -> &'static str {
        match self {
            Self::BadString => "<bad-string>",
            Self::BadUrl => "<bad-url>",
            Self::Whitespace => " ",
            Self::Colon => ":",
            Self::Semicolon => ";",
            Self::Comma => ",",
            Self::OpenBrace => "{",
            Self::CloseBrace => "}",
            Self::OpenParenthesis => "(",
            Self::CloseParenthesis => ")",
            Self::OpenBracket => "[",
            Self::CloseBracket => "]",
            _ => "",
        }
    }
}

/// A token and the position of its first character.
#[derive(Clone, Debug, PartialEq)]
pub struct SpannedToken {
    token: Token,
    span: SourceSpan,
}

impl SpannedToken {
    #[must_use]
    pub const fn new(token: Token, span: SourceSpan) -> Self {
        Self { token, span }
    }

    #[must_use]
    pub const fn token(&self) -> &Token {
        &self.token
    }

    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// The tokens of one stylesheet plus a read cursor. A first-class collection —
/// no public `Vec` (`ADR-0010:129`).
///
/// Command–Query Separation is kept strictly: [`TokenStream::peek`] and
/// [`TokenStream::peek_span`] answer and mutate nothing;
/// [`TokenStream::advance`] and [`TokenStream::skip_whitespace`] mutate and
/// return `()`.
#[derive(Clone, Debug, PartialEq)]
pub struct TokenStream {
    tokens: Vec<SpannedToken>,
    position: usize,
    end: SourceSpan,
}

impl TokenStream {
    #[must_use]
    pub const fn new(tokens: Vec<SpannedToken>, end: SourceSpan) -> Self {
        Self {
            tokens,
            position: 0,
            end,
        }
    }

    /// The token under the cursor.
    #[must_use]
    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position).map(SpannedToken::token)
    }

    /// The token `offset` places after the cursor.
    #[must_use]
    pub fn peek_ahead(&self, offset: usize) -> Option<&Token> {
        self.position
            .checked_add(offset)
            .and_then(|index| self.tokens.get(index))
            .map(SpannedToken::token)
    }

    /// Where the token under the cursor began — the end of the source once the
    /// stream is exhausted, so an error raised at EOF still has a location.
    #[must_use]
    pub fn peek_span(&self) -> SourceSpan {
        self.tokens
            .get(self.position)
            .map_or(self.end, SpannedToken::span)
    }

    /// Moves past the token under the cursor.
    pub const fn advance(&mut self) {
        self.position = self.position.saturating_add(1);
    }

    /// Moves past a run of whitespace tokens, if the cursor is on one.
    pub fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(Token::is_whitespace) {
            self.advance();
        }
    }

    #[must_use]
    pub const fn is_exhausted(&self) -> bool {
        self.position >= self.tokens.len()
    }

    /// Every token, ignoring the cursor — what a tokenizer test asserts on.
    pub fn iter(&self) -> impl Iterator<Item = &SpannedToken> + '_ {
        self.tokens.iter()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.tokens.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}
