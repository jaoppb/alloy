//! Rules, declarations and the recovery discipline of CSS Syntax Level 3 §5.4.
//!
//! Recovery is the whole point of this file. A malformed rule is consumed up to
//! the `}` that closes it, a malformed declaration up to its `;`, and parsing
//! continues — one bad selector never costs the rest of the sheet. What it does
//! cost is a [`ParseNote`] carrying the `SourceSpan` where it happened, because
//! recovering *silently* is how a declared cut shrinks unnoticed
//! (`relatório §2.8:350-354`).
//!
//! Exactly one failure is not recoverable and travels out as a `CssError`:
//! block nesting past [`MAX_NESTING_DEPTH`]. A source that deep is not a
//! stylesheet, it is the hostile input the fuzz budget of §2.11 exists for, and
//! refusing it whole is the correct answer.

use crate::domain::declaration::{Declaration, DeclarationBlock, DeclarationValue, Importance};
use crate::domain::error::{CssError, CssStage, SourceSpan};
use crate::domain::identifier::Identifier;
use crate::domain::media::MediaQuery;
use crate::domain::parse_notes::ParseNote;
use crate::domain::stylesheet_set::{Origin, StyleRule, StyleSheetSet};
use crate::infrastructure::parser::media::parse_media_prelude;
use crate::infrastructure::parser::selectors::parse_selector_list;
use crate::infrastructure::parser::token::{Token, TokenStream};

/// How deeply `{`, `(` and `[` may nest before the source is refused whole.
pub(crate) const MAX_NESTING_DEPTH: usize = 32;

/// Whether the rule list being read is the sheet itself or the body of an
/// `@media` block — which is what a `}` means at that point.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Nesting {
    /// The stylesheet's own rule list; a `}` here is stray.
    TopLevel,
    /// The body of an `@media` block; a `}` here ends it.
    Block,
}

/// Whether a rule list keeps reading after the token just handled.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ListState {
    /// More rules may follow.
    Continue,
    /// The list is over — the `}` that closed it was consumed.
    Finished,
}

/// Reads every rule of `tokens` into `sheets`, tagged with `origin`.
pub(crate) fn parse_rule_list(
    tokens: &mut TokenStream,
    origin: Origin,
    sheets: &mut StyleSheetSet,
) -> Result<(), CssError> {
    read_rules(
        tokens,
        origin,
        &MediaQuery::always(),
        Nesting::TopLevel,
        sheets,
    )
}

fn read_rules(
    tokens: &mut TokenStream,
    origin: Origin,
    media: &MediaQuery,
    nesting: Nesting,
    sheets: &mut StyleSheetSet,
) -> Result<(), CssError> {
    loop {
        tokens.skip_whitespace();
        let span = tokens.peek_span();
        let Some(token) = tokens.peek().cloned() else {
            note_unterminated_block(nesting, span, sheets);
            return Ok(());
        };
        let Some(state) = brace_state(tokens, nesting, &token, span, sheets) else {
            read_one_rule(tokens, origin, media, &token, sheets)?;
            continue;
        };
        if state == ListState::Finished {
            return Ok(());
        }
    }
}

fn read_one_rule(
    tokens: &mut TokenStream,
    origin: Origin,
    media: &MediaQuery,
    token: &Token,
    sheets: &mut StyleSheetSet,
) -> Result<(), CssError> {
    let Token::AtKeyword(name) = token else {
        return qualified_rule(tokens, origin, media, sheets);
    };
    at_rule(tokens, origin, media, name, sheets)
}

/// End of source inside an `@media` body: recoverable, but worth saying.
fn note_unterminated_block(nesting: Nesting, span: SourceSpan, sheets: &mut StyleSheetSet) {
    if nesting == Nesting::Block {
        sheets.push_note(ParseNote::new("unterminated `@media` block", span));
    }
}

/// A `}` ends an `@media` body. At the top level it is **stray**: CSS Syntax L3
/// §5.4.1 drops it and keeps reading, so one unbalanced brace never costs the
/// rules that follow it.
///
/// `None` when `token` is not a `}` at all — the caller then reads a rule.
fn brace_state(
    tokens: &mut TokenStream,
    nesting: Nesting,
    token: &Token,
    span: SourceSpan,
    sheets: &mut StyleSheetSet,
) -> Option<ListState> {
    if token != &Token::CloseBrace {
        return None;
    }
    tokens.advance();
    if nesting == Nesting::Block {
        return Some(ListState::Finished);
    }
    sheets.push_note(ParseNote::new("stray `}` skipped", span));
    Some(ListState::Continue)
}

// ---- at-rules -----------------------------------------------------------

/// `@media` is the only at-rule of the v0.5 cut (`relatório §2.8:344`);
/// `@supports`, `@font-face`, `@import` and `@keyframes` are declared out and
/// are skipped with a note rather than half-applied.
fn at_rule(
    tokens: &mut TokenStream,
    origin: Origin,
    media: &MediaQuery,
    name: &str,
    sheets: &mut StyleSheetSet,
) -> Result<(), CssError> {
    let span = tokens.peek_span();
    tokens.advance();
    if !name.eq_ignore_ascii_case("media") || !media.is_always() {
        sheets.push_note(ParseNote::new(unsupported_at_rule(name, media), span));
        return skip_at_rule(tokens, sheets);
    }
    media_rule(tokens, origin, span, sheets)
}

fn unsupported_at_rule(name: &str, media: &MediaQuery) -> String {
    if media.is_always() {
        return format!("`@{name}` is outside the v0.5 cut (see tests/data/MANIFEST.md)");
    }
    format!("`@{name}` nested inside `@media` is outside the v0.5 cut")
}

fn media_rule(
    tokens: &mut TokenStream,
    origin: Origin,
    span: SourceSpan,
    sheets: &mut StyleSheetSet,
) -> Result<(), CssError> {
    tokens.skip_whitespace();
    let query = match parse_media_prelude(tokens) {
        Ok(query) => query,
        Err(error) => return skip_unreadable(tokens, &error, span, sheets),
    };
    if tokens.peek() != Some(&Token::OpenBrace) {
        sheets.push_note(ParseNote::new("`@media` prelude has no block", span));
        return skip_at_rule(tokens, sheets);
    }
    tokens.advance();
    read_rules(tokens, origin, &query, Nesting::Block, sheets)
}

/// Records why a construct was refused, then consumes it.
fn skip_unreadable(
    tokens: &mut TokenStream,
    error: &CssError,
    span: SourceSpan,
    sheets: &mut StyleSheetSet,
) -> Result<(), CssError> {
    sheets.push_note(ParseNote::new(error.to_string(), span));
    skip_at_rule(tokens, sheets)
}

/// Consumes the rest of a rule: its block if it has one, otherwise up to and
/// including its `;` (CSS Syntax L3 §5.4.2).
fn skip_at_rule(tokens: &mut TokenStream, sheets: &mut StyleSheetSet) -> Result<(), CssError> {
    while let Some(token) = tokens.peek().cloned() {
        if token == Token::CloseBrace {
            return Ok(());
        }
        if token == Token::OpenBrace {
            return skip_block(tokens, sheets);
        }
        tokens.advance();
        if token == Token::Semicolon {
            return Ok(());
        }
    }
    Ok(())
}

/// Consumes a balanced `{ … }`, refusing a source nested past
/// [`MAX_NESTING_DEPTH`].
fn skip_block(tokens: &mut TokenStream, sheets: &mut StyleSheetSet) -> Result<(), CssError> {
    let span = tokens.peek_span();
    let mut depth: usize = 0;
    while let Some(token) = tokens.peek().cloned() {
        depth = adjust_depth(depth, &token, span)?;
        tokens.advance();
        if depth == 0 {
            return Ok(());
        }
    }
    sheets.push_note(ParseNote::new("unterminated block skipped", span));
    Ok(())
}

fn adjust_depth(depth: usize, token: &Token, span: SourceSpan) -> Result<usize, CssError> {
    if token.opens().is_some() {
        return deepen(depth, span);
    }
    if token.closes().is_some() {
        return Ok(depth.saturating_sub(1));
    }
    Ok(depth)
}

fn deepen(depth: usize, span: SourceSpan) -> Result<usize, CssError> {
    let deeper = depth.saturating_add(1);
    if deeper > MAX_NESTING_DEPTH {
        return Err(CssError::unsupported(
            CssStage::Parse,
            format!("block nesting deeper than {MAX_NESTING_DEPTH} is refused"),
        )
        .with_span(span));
    }
    Ok(deeper)
}

// ---- qualified rules ----------------------------------------------------

/// `selector-list { declarations }`.
fn qualified_rule(
    tokens: &mut TokenStream,
    origin: Origin,
    media: &MediaQuery,
    sheets: &mut StyleSheetSet,
) -> Result<(), CssError> {
    let span = tokens.peek_span();
    let selectors = match parse_selector_list(tokens) {
        Ok(selectors) => selectors,
        Err(error) => return skip_unreadable(tokens, &error, span, sheets),
    };
    if tokens.peek() != Some(&Token::OpenBrace) {
        sheets.push_note(ParseNote::new("a rule needs a `{` block", span));
        return skip_at_rule(tokens, sheets);
    }
    tokens.advance();
    let declarations = read_declaration_block(tokens, sheets)?;
    let rule = StyleRule::new(selectors, declarations).with_media(media.clone());
    sheets.push_rule(origin, rule);
    Ok(())
}

/// The declarations between `{` and `}`, with the `}` consumed. Also the body
/// of a `style=` attribute, which has neither brace.
pub(crate) fn read_declaration_block(
    tokens: &mut TokenStream,
    sheets: &mut StyleSheetSet,
) -> Result<DeclarationBlock, CssError> {
    let mut block = DeclarationBlock::new();
    loop {
        tokens.skip_whitespace();
        let span = tokens.peek_span();
        let Some(token) = tokens.peek().cloned() else {
            return Ok(block);
        };
        if token == Token::CloseBrace {
            tokens.advance();
            return Ok(block);
        }
        read_block_entry(tokens, &token, span, &mut block, sheets)?;
    }
}

fn read_block_entry(
    tokens: &mut TokenStream,
    token: &Token,
    span: SourceSpan,
    block: &mut DeclarationBlock,
    sheets: &mut StyleSheetSet,
) -> Result<(), CssError> {
    if token == &Token::Semicolon {
        tokens.advance();
        return Ok(());
    }
    let Token::Ident(name) = token else {
        sheets.push_note(ParseNote::new("a declaration needs a property name", span));
        skip_to_declaration_end(tokens);
        return Ok(());
    };
    read_declaration(tokens, name, span, block, sheets)
}

/// One `property: value` pair. An unsupported property, a missing `:` or an
/// unreadable value drops **this declaration only**, with a note.
fn read_declaration(
    tokens: &mut TokenStream,
    name: &str,
    span: SourceSpan,
    block: &mut DeclarationBlock,
    sheets: &mut StyleSheetSet,
) -> Result<(), CssError> {
    tokens.advance();
    tokens.skip_whitespace();
    if tokens.peek() != Some(&Token::Colon) {
        sheets.push_note(ParseNote::new(format!("`{name}` has no `:` value"), span));
        skip_to_declaration_end(tokens);
        return Ok(());
    }
    tokens.advance();
    let value = read_declaration_value(tokens, sheets)?;
    push_declaration(name, &value, span, block, sheets);
    Ok(())
}

fn push_declaration(
    name: &str,
    value: &str,
    span: SourceSpan,
    block: &mut DeclarationBlock,
    sheets: &mut StyleSheetSet,
) {
    let Some(property) = supported_property(name) else {
        sheets.push_note(ParseNote::new(
            format!("`{name}` is outside the v0.5 property cut (see tests/data/MANIFEST.md)"),
            span,
        ));
        return;
    };
    let (text, importance) = split_importance(value);
    block.push(Declaration::new(
        property,
        DeclarationValue::new(text),
        importance,
    ));
}

/// The property, if `crate::SUPPORTED_PROPERTIES` declares it. That list is the
/// single registry `tests/data/MANIFEST.md` is checked against, so a property
/// accepted here is a property the manifest names.
fn supported_property(name: &str) -> Option<Identifier> {
    let lowered = name.to_ascii_lowercase();
    if !crate::SUPPORTED_PROPERTIES.contains(&lowered.as_str()) {
        return None;
    }
    Identifier::lowercased(&lowered)
}

/// Splits a trailing `!important` off a value (CSS Cascade L4 §6.2).
fn split_importance(value: &str) -> (&str, Importance) {
    let trimmed = value.trim();
    let Some(head) = strip_important(trimmed) else {
        return (trimmed, Importance::Normal);
    };
    (head.trim_end(), Importance::Important)
}

fn strip_important(value: &str) -> Option<&str> {
    let (head, tail) = value.rsplit_once('!')?;
    tail.trim()
        .eq_ignore_ascii_case("important")
        .then_some(head)
}

/// The value text up to the `;` or `}` that ends the declaration, rebuilt from
/// its tokens. Balanced `(` / `[` nesting is respected, so `url(a;b)` survives.
fn read_declaration_value(
    tokens: &mut TokenStream,
    sheets: &mut StyleSheetSet,
) -> Result<String, CssError> {
    let span = tokens.peek_span();
    let mut text = String::new();
    let mut depth: usize = 0;
    while let Some(token) = tokens.peek().cloned() {
        if ends_declaration(&token, depth) {
            return Ok(finish_declaration_value(tokens, text, sheets));
        }
        depth = adjust_depth(depth, &token, span)?;
        tokens.advance();
        text.push_str(&token.to_string());
    }
    note_bad_tokens(&text, span, sheets);
    Ok(text)
}

const fn ends_declaration(token: &Token, depth: usize) -> bool {
    depth == 0 && matches!(token, Token::Semicolon | Token::CloseBrace)
}

/// Consumes the `;` (but never the `}`, which the block loop needs to see).
fn finish_declaration_value(
    tokens: &mut TokenStream,
    text: String,
    sheets: &mut StyleSheetSet,
) -> String {
    let span = tokens.peek_span();
    if tokens.peek() == Some(&Token::Semicolon) {
        tokens.advance();
    }
    note_bad_tokens(&text, span, sheets);
    text
}

/// A value that carried an unterminated string or url still parses — as a value
/// that will not resolve. Say so rather than letting it fail quietly later.
fn note_bad_tokens(text: &str, span: SourceSpan, sheets: &mut StyleSheetSet) {
    if !text.contains("<bad-string>") && !text.contains("<bad-url>") {
        return;
    }
    sheets.push_note(ParseNote::new(
        "value contains an unterminated string or url",
        span,
    ));
}

/// Recovers at the next `;`, leaving a `}` for the block loop.
fn skip_to_declaration_end(tokens: &mut TokenStream) {
    while let Some(token) = tokens.peek().cloned() {
        if token == Token::CloseBrace {
            return;
        }
        tokens.advance();
        if token == Token::Semicolon {
            return;
        }
    }
}
