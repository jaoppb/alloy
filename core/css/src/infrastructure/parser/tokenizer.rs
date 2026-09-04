//! [`tokenize`] — the CSS Syntax Level 3 §4 tokenizer.
//!
//! It is **total**: no input makes it fail. That is the spec's design and it is
//! also what keeps a single stray quote from costing the rest of the sheet — an
//! unterminated string becomes [`Token::BadString`], an unreadable `url(`
//! becomes [`Token::BadUrl`], and the rule parser turns each into a
//! `ParseNote` with a span while it recovers (`rules.rs`, CSS Syntax L3 §5.4).
//! Nothing is dropped quietly.
//!
//! One deviation from §4.3.5, deliberate: the spec ends a string at EOF with a
//! *parse error* and returns the string token anyway. Returning
//! [`Token::BadString`] instead keeps that parse error visible in the notes;
//! the recovery is identical.

use crate::domain::error::SourceSpan;
use crate::infrastructure::parser::scanner::Scanner;
use crate::infrastructure::parser::token::{SpannedToken, Token, TokenStream};

/// The most hex digits a `\` escape may carry (CSS Syntax L3 §4.3.7).
const MAX_ESCAPE_DIGITS: usize = 6;

/// Turns a stylesheet into its tokens. Never fails.
#[must_use]
pub fn tokenize(source: &str) -> TokenStream {
    let mut scanner = Scanner::new(source);
    let mut tokens: Vec<SpannedToken> = Vec::new();
    while !scanner.is_exhausted() {
        let span = scanner.span();
        let produced = next_token(&mut scanner);
        push_token(&mut tokens, produced, span);
    }
    TokenStream::new(tokens, scanner.span())
}

fn push_token(tokens: &mut Vec<SpannedToken>, produced: Option<Token>, span: SourceSpan) {
    let Some(token) = produced else {
        return;
    };
    tokens.push(SpannedToken::new(token, span));
}

/// Reads one token, or `None` for a construct that produces none (a comment,
/// `<!--`, `-->`). Always consumes at least one character, so [`tokenize`]
/// terminates.
fn next_token(scanner: &mut Scanner) -> Option<Token> {
    let character = scanner.peek()?;
    match character {
        _ if is_whitespace(character) => Some(read_whitespace(scanner)),
        '/' if scanner.peek_ahead(1) == Some('*') => skip_comment(scanner),
        '<' if is_comment_open(scanner) => skip_markup_comment(scanner, 4),
        '-' if is_comment_close(scanner) => skip_markup_comment(scanner, 3),
        '"' | '\'' => Some(read_string(scanner, character)),
        '#' => Some(read_hash(scanner)),
        '@' => Some(read_at_keyword(scanner)),
        _ if starts_number(scanner) => Some(read_numeric(scanner)),
        _ if starts_identifier(scanner) => Some(read_identifier_like(scanner)),
        _ => Some(read_punctuation(scanner, character)),
    }
}

// ---- character classes (CSS Syntax L3 §4.2) ------------------------------

const fn is_whitespace(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\n')
}

fn is_identifier_start(character: char) -> bool {
    character.is_alphabetic() || character == '_' || !character.is_ascii()
}

fn is_identifier_char(character: char) -> bool {
    is_identifier_start(character) || character.is_ascii_digit() || character == '-'
}

/// Whether the cursor sits on something that can begin an identifier: a letter,
/// `_`, a non-ASCII character, an escape, or a `-` leading any of those.
fn starts_identifier(scanner: &Scanner) -> bool {
    let Some(character) = scanner.peek() else {
        return false;
    };
    if character == '-' {
        return leads_identifier_after_dash(scanner);
    }
    is_identifier_start(character) || character == '\\'
}

fn leads_identifier_after_dash(scanner: &Scanner) -> bool {
    scanner
        .peek_ahead(1)
        .is_some_and(|next| is_identifier_start(next) || matches!(next, '-' | '\\'))
}

/// Whether the cursor sits on a numeric literal (CSS Syntax L3 §4.3.10).
fn starts_number(scanner: &Scanner) -> bool {
    let Some(character) = scanner.peek() else {
        return false;
    };
    match character {
        _ if character.is_ascii_digit() => true,
        '.' => scanner
            .peek_ahead(1)
            .is_some_and(|next| next.is_ascii_digit()),
        '+' | '-' => leads_number_after_sign(scanner),
        _ => false,
    }
}

fn leads_number_after_sign(scanner: &Scanner) -> bool {
    let Some(next) = scanner.peek_ahead(1) else {
        return false;
    };
    if next.is_ascii_digit() {
        return true;
    }
    next == '.'
        && scanner
            .peek_ahead(2)
            .is_some_and(|third| third.is_ascii_digit())
}

fn is_comment_open(scanner: &Scanner) -> bool {
    scanner.peek_ahead(1) == Some('!')
        && scanner.peek_ahead(2) == Some('-')
        && scanner.peek_ahead(3) == Some('-')
}

fn is_comment_close(scanner: &Scanner) -> bool {
    scanner.peek_ahead(1) == Some('-') && scanner.peek_ahead(2) == Some('>')
}

// ---- readers, one construct each ----------------------------------------

fn read_whitespace(scanner: &mut Scanner) -> Token {
    while scanner.peek().is_some_and(is_whitespace) {
        scanner.consume();
    }
    Token::Whitespace
}

/// `/* … */`. An unterminated comment runs to the end of the source, which is
/// what CSS Syntax L3 §4.3.2 asks for.
fn skip_comment(scanner: &mut Scanner) -> Option<Token> {
    scanner.consume();
    scanner.consume();
    while let Some(character) = scanner.peek() {
        if character == '*' && scanner.peek_ahead(1) == Some('/') {
            scanner.consume();
            scanner.consume();
            return None;
        }
        scanner.consume();
    }
    None
}

/// `<!--` and `-->`, which old stylesheets wrap themselves in when they live
/// inside a `<style>` element. They carry no meaning; skipping them is what
/// CSS Syntax L3 §5.3.3 does at the top level.
fn skip_markup_comment(scanner: &mut Scanner, characters: usize) -> Option<Token> {
    for _step in 0..characters {
        scanner.consume();
    }
    None
}

/// A quoted string, escapes resolved, quotes removed.
fn read_string(scanner: &mut Scanner, quote: char) -> Token {
    scanner.consume();
    let mut text = String::new();
    while let Some(character) = scanner.peek() {
        match character {
            _ if character == quote => {
                scanner.consume();
                return Token::QuotedString(text);
            }
            '\n' => return Token::BadString,
            '\\' => push_escape(scanner, &mut text),
            _ => push_literal(scanner, &mut text, character),
        }
    }
    Token::BadString
}

fn push_literal(scanner: &mut Scanner, text: &mut String, character: char) {
    scanner.consume();
    text.push(character);
}

fn push_escape(scanner: &mut Scanner, text: &mut String) {
    let Some(character) = read_escape(scanner) else {
        return;
    };
    text.push(character);
}

/// `\` followed by up to six hex digits and one optional whitespace, or by any
/// single character (CSS Syntax L3 §4.3.7). A `\` at end of source resolves to
/// `U+FFFD`, as the spec requires.
fn read_escape(scanner: &mut Scanner) -> Option<char> {
    scanner.consume();
    let Some(character) = scanner.peek() else {
        return Some('\u{FFFD}');
    };
    if !character.is_ascii_hexdigit() {
        scanner.consume();
        return Some(character);
    }
    read_hex_escape(scanner)
}

fn read_hex_escape(scanner: &mut Scanner) -> Option<char> {
    let mut digits = String::new();
    while digits.len() < MAX_ESCAPE_DIGITS {
        let Some(digit) = scanner.peek().filter(char::is_ascii_hexdigit) else {
            break;
        };
        scanner.consume();
        digits.push(digit);
    }
    consume_escape_whitespace(scanner);
    u32::from_str_radix(&digits, 16)
        .ok()
        .and_then(char::from_u32)
        .or(Some('\u{FFFD}'))
}

fn consume_escape_whitespace(scanner: &mut Scanner) {
    if scanner.peek().is_some_and(is_whitespace) {
        scanner.consume();
    }
}

/// The value of an identifier: identifier characters and escapes, nothing else.
fn read_name(scanner: &mut Scanner) -> String {
    let mut text = String::new();
    while let Some(character) = scanner.peek() {
        match character {
            '\\' => push_escape(scanner, &mut text),
            _ if is_identifier_char(character) => push_literal(scanner, &mut text, character),
            _ => return text,
        }
    }
    text
}

/// `#main`, `#0af`. A `#` with no name after it is a plain delimiter.
fn read_hash(scanner: &mut Scanner) -> Token {
    scanner.consume();
    let name = read_name(scanner);
    if name.is_empty() {
        return Token::Delimiter('#');
    }
    Token::Hash(name)
}

/// `@media`. An `@` with no identifier after it is a plain delimiter.
fn read_at_keyword(scanner: &mut Scanner) -> Token {
    scanner.consume();
    if !starts_identifier(scanner) {
        return Token::Delimiter('@');
    }
    Token::AtKeyword(read_name(scanner))
}

/// An identifier, a function, or a `url()` — the three things a name can start
/// (CSS Syntax L3 §4.3.4).
fn read_identifier_like(scanner: &mut Scanner) -> Token {
    let name = read_name(scanner);
    if scanner.peek() != Some('(') {
        return Token::Ident(name);
    }
    scanner.consume();
    if name.eq_ignore_ascii_case("url") && !starts_quoted_url(scanner) {
        return read_url(scanner);
    }
    Token::Function(name)
}

/// `url("x")` is a function token holding a string; only `url(x)` is a url
/// token (CSS Syntax L3 §4.3.4).
fn starts_quoted_url(scanner: &Scanner) -> bool {
    let mut offset = 0;
    while scanner.peek_ahead(offset).is_some_and(is_whitespace) {
        offset = offset.saturating_add(1);
    }
    matches!(scanner.peek_ahead(offset), Some('"' | '\''))
}

/// The unquoted payload of `url(`, up to its `)`.
fn read_url(scanner: &mut Scanner) -> Token {
    skip_whitespace(scanner);
    let mut text = String::new();
    while let Some(character) = scanner.peek() {
        match character {
            ')' => {
                scanner.consume();
                return Token::Url(text);
            }
            '"' | '\'' | '(' => return consume_bad_url(scanner),
            _ if is_whitespace(character) => return finish_spaced_url(scanner, text),
            '\\' => push_escape(scanner, &mut text),
            _ => push_literal(scanner, &mut text, character),
        }
    }
    Token::BadUrl
}

fn skip_whitespace(scanner: &mut Scanner) {
    while scanner.peek().is_some_and(is_whitespace) {
        scanner.consume();
    }
}

/// Trailing whitespace inside `url( x )` is allowed; anything else after it is
/// not.
fn finish_spaced_url(scanner: &mut Scanner, text: String) -> Token {
    skip_whitespace(scanner);
    if scanner.peek() != Some(')') {
        return consume_bad_url(scanner);
    }
    scanner.consume();
    Token::Url(text)
}

/// Consumes up to and including the `)` that ends a malformed url, so the rest
/// of the sheet still tokenizes (CSS Syntax L3 §4.3.14).
fn consume_bad_url(scanner: &mut Scanner) -> Token {
    while let Some(character) = scanner.peek() {
        scanner.consume();
        if character == ')' {
            return Token::BadUrl;
        }
    }
    Token::BadUrl
}

/// A number, a dimension or a percentage (CSS Syntax L3 §4.3.3).
fn read_numeric(scanner: &mut Scanner) -> Token {
    let value = read_number_value(scanner);
    if scanner.peek() == Some('%') {
        scanner.consume();
        return Token::Percentage(value);
    }
    if starts_identifier(scanner) {
        return Token::Dimension(value, read_name(scanner));
    }
    Token::Number(value)
}

/// The numeric literal itself: sign, integer part, fraction, exponent.
///
/// The digits are gathered as text and handed to `f32::from_str`, which is the
/// only conversion that neither casts (`as_conversions` is denied) nor does
/// arithmetic on the accumulator (`arithmetic_side_effects` is denied). A
/// literal too large for `f32` parses to infinity, which
/// [`crate::Length::resolve_to_au`] then refuses — a non-finite length has no
/// correct reading.
fn read_number_value(scanner: &mut Scanner) -> f32 {
    let mut literal = String::new();
    push_sign(scanner, &mut literal);
    push_digits(scanner, &mut literal);
    push_fraction(scanner, &mut literal);
    push_exponent(scanner, &mut literal);
    literal.parse::<f32>().unwrap_or_default()
}

fn push_sign(scanner: &mut Scanner, literal: &mut String) {
    let Some(sign) = scanner.peek().filter(|next| matches!(*next, '+' | '-')) else {
        return;
    };
    push_literal(scanner, literal, sign);
}

fn push_digits(scanner: &mut Scanner, literal: &mut String) {
    while let Some(digit) = scanner.peek().filter(char::is_ascii_digit) {
        push_literal(scanner, literal, digit);
    }
}

fn push_fraction(scanner: &mut Scanner, literal: &mut String) {
    let fractional = scanner.peek() == Some('.')
        && scanner
            .peek_ahead(1)
            .is_some_and(|next| next.is_ascii_digit());
    if !fractional {
        return;
    }
    push_literal(scanner, literal, '.');
    push_digits(scanner, literal);
}

fn push_exponent(scanner: &mut Scanner, literal: &mut String) {
    let Some(marker) = scanner.peek().filter(|next| matches!(*next, 'e' | 'E')) else {
        return;
    };
    if !exponent_follows(scanner) {
        return;
    }
    push_literal(scanner, literal, marker);
    push_sign(scanner, literal);
    push_digits(scanner, literal);
}

fn exponent_follows(scanner: &Scanner) -> bool {
    let Some(after) = scanner.peek_ahead(1) else {
        return false;
    };
    if after.is_ascii_digit() {
        return true;
    }
    matches!(after, '+' | '-')
        && scanner
            .peek_ahead(2)
            .is_some_and(|next| next.is_ascii_digit())
}

/// Everything else: the structural punctuation, and any leftover character as a
/// delimiter for the parser above to accept or refuse.
fn read_punctuation(scanner: &mut Scanner, character: char) -> Token {
    scanner.consume();
    match character {
        ':' => Token::Colon,
        ';' => Token::Semicolon,
        ',' => Token::Comma,
        '{' => Token::OpenBrace,
        '}' => Token::CloseBrace,
        '(' => Token::OpenParenthesis,
        ')' => Token::CloseParenthesis,
        '[' => Token::OpenBracket,
        ']' => Token::CloseBracket,
        _ => Token::Delimiter(character),
    }
}
