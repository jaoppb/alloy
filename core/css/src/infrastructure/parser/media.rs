//! The `@media` prelude of the v0.5 cut: `min-width` and `max-width`, joined by
//! `and` (`relatório §2.8:344`).
//!
//! Everything else a media query can say — media types, `not`, `only`, `,`,
//! ranges, `orientation`, `prefers-*` — is refused with a `CssError`, which
//! makes `rules.rs` skip the whole `@media` block and record a note. Applying
//! the rules of a query we could not read would be worse than dropping them.

use crate::domain::error::{CssError, CssStage, SourceSpan};
use crate::domain::length::Length;
use crate::domain::media::{MediaCondition, MediaFeature, MediaQuery};
use crate::infrastructure::parser::token::{Token, TokenStream};
use crate::infrastructure::parser::values::length_from_token;

/// Parses the prelude between `@media` and its `{`, leaving the cursor on the
/// `{`.
pub(crate) fn parse_media_prelude(tokens: &mut TokenStream) -> Result<MediaQuery, CssError> {
    let mut query = MediaQuery::always();
    query.push(parse_condition(tokens)?);
    tokens.skip_whitespace();
    while is_conjunction(tokens) {
        tokens.advance();
        query.push(parse_condition(tokens)?);
        tokens.skip_whitespace();
    }
    Ok(query)
}

/// Whether an `and` joins another condition to the ones already read. A pure
/// query — the caller commits by advancing.
fn is_conjunction(tokens: &TokenStream) -> bool {
    let Some(Token::Ident(name)) = tokens.peek() else {
        return false;
    };
    name.eq_ignore_ascii_case("and")
}

/// One `(feature: length)` pair.
fn parse_condition(tokens: &mut TokenStream) -> Result<MediaCondition, CssError> {
    tokens.skip_whitespace();
    let span = tokens.peek_span();
    if tokens.peek() != Some(&Token::OpenParenthesis) {
        return Err(media_error("a media condition must be parenthesised", span));
    }
    tokens.advance();
    tokens.skip_whitespace();
    let feature = parse_feature(tokens, span)?;
    expect_colon(tokens, span)?;
    let length = parse_condition_length(tokens, span)?;
    close_condition(tokens, span)?;
    Ok(MediaCondition::new(feature, length))
}

fn parse_feature(tokens: &mut TokenStream, span: SourceSpan) -> Result<MediaFeature, CssError> {
    let Some(Token::Ident(name)) = tokens.peek().cloned() else {
        return Err(media_error("expected a media feature name", span));
    };
    let feature = media_feature(&name.to_ascii_lowercase()).ok_or_else(|| {
        media_error(
            format!("`{name}` is outside the v0.5 media-feature cut"),
            span,
        )
    })?;
    tokens.advance();
    Ok(feature)
}

fn media_feature(lowered: &str) -> Option<MediaFeature> {
    match lowered {
        "min-width" => Some(MediaFeature::MinWidth),
        "max-width" => Some(MediaFeature::MaxWidth),
        _ => None,
    }
}

fn expect_colon(tokens: &mut TokenStream, span: SourceSpan) -> Result<(), CssError> {
    tokens.skip_whitespace();
    if tokens.peek() != Some(&Token::Colon) {
        return Err(media_error("a media feature needs a `:` value", span));
    }
    tokens.advance();
    tokens.skip_whitespace();
    Ok(())
}

fn parse_condition_length(tokens: &mut TokenStream, span: SourceSpan) -> Result<Length, CssError> {
    let Some(token) = tokens.peek().cloned() else {
        return Err(media_error("a media feature needs a length", span));
    };
    let length = length_from_token(&token)
        .ok_or_else(|| media_error("a media feature value must be a length", span))?;
    tokens.advance();
    Ok(length)
}

fn close_condition(tokens: &mut TokenStream, span: SourceSpan) -> Result<(), CssError> {
    tokens.skip_whitespace();
    if tokens.peek() != Some(&Token::CloseParenthesis) {
        return Err(media_error("unterminated media condition", span));
    }
    tokens.advance();
    Ok(())
}

fn media_error(detail: impl Into<String>, span: SourceSpan) -> CssError {
    CssError::unsupported(CssStage::Parse, detail).with_span(span)
}
