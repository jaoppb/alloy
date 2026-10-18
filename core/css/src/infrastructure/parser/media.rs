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
use crate::infrastructure::parser::tokenizer::tokenize;
use crate::infrastructure::parser::values::length_from_token;

/// Parses the `media` attribute of a `<style>` or `<link>` element
/// (HTML §4.2.4, §4.2.6).
///
/// Missing or empty attributes, `all` and `screen` match unconditionally
/// (`MediaQuery::always()`). Bare media types outside screen/all (`print`, `speech`)
/// and unparseable media expressions never match on screen (`MediaQuery::never()`).
#[must_use]
pub fn parse_media_attribute(raw: Option<&str>) -> MediaQuery {
    let Some(raw) = raw else {
        return MediaQuery::always();
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return MediaQuery::always();
    }
    let mut tokens = tokenize(trimmed);
    tokens.skip_whitespace();
    let Some(token) = tokens.peek().cloned() else {
        return MediaQuery::always();
    };
    parse_tokens_for_media_attribute(&mut tokens, token)
}

fn parse_tokens_for_media_attribute(tokens: &mut TokenStream, token: Token) -> MediaQuery {
    match token {
        Token::Ident(name) => parse_ident_media_attribute(tokens, &name),
        Token::OpenParenthesis => parse_conditions_and_ensure_exhausted(tokens),
        _ => MediaQuery::never(),
    }
}

fn parse_ident_media_attribute(tokens: &mut TokenStream, name: &str) -> MediaQuery {
    tokens.advance();
    tokens.skip_whitespace();
    if tokens.is_exhausted() {
        if is_screen_or_all(name) {
            return MediaQuery::always();
        }
        return MediaQuery::never();
    }
    if !is_screen_or_all(name) || !is_conjunction(tokens) {
        return MediaQuery::never();
    }
    tokens.advance();
    tokens.skip_whitespace();
    parse_conditions_and_ensure_exhausted(tokens)
}

const fn is_screen_or_all(name: &str) -> bool {
    name.eq_ignore_ascii_case("screen") || name.eq_ignore_ascii_case("all")
}

fn parse_conditions_and_ensure_exhausted(tokens: &mut TokenStream) -> MediaQuery {
    let Ok(query) = parse_media_prelude(tokens) else {
        return MediaQuery::never();
    };
    tokens.skip_whitespace();
    if !tokens.is_exhausted() {
        return MediaQuery::never();
    }
    query
}

/// Parses the prelude between `@media` and its `{`, leaving the cursor on the
/// `{`.
pub(crate) fn parse_media_prelude(tokens: &mut TokenStream) -> Result<MediaQuery, CssError> {
    let mut conditions = vec![parse_condition(tokens)?];
    tokens.skip_whitespace();
    while is_conjunction(tokens) {
        tokens.advance();
        conditions.push(parse_condition(tokens)?);
        tokens.skip_whitespace();
    }
    Ok(MediaQuery::Conditions(conditions))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_missing_or_empty_as_always() {
        assert!(parse_media_attribute(None).is_always());
        assert!(parse_media_attribute(Some("")).is_always());
        assert!(parse_media_attribute(Some("   \t\n  ")).is_always());
    }

    #[test]
    fn parses_bare_media_types() {
        assert!(parse_media_attribute(Some("all")).is_always());
        assert!(parse_media_attribute(Some("screen")).is_always());
        assert!(parse_media_attribute(Some("SCREEN")).is_always());
        assert!(parse_media_attribute(Some("  ALL  ")).is_always());

        assert!(parse_media_attribute(Some("print")).is_never());
        assert!(parse_media_attribute(Some("speech")).is_never());
        assert!(parse_media_attribute(Some("projection")).is_never());
    }

    #[test]
    fn parses_parenthesised_conditions() {
        let query = parse_media_attribute(Some("(min-width: 300px)"));
        assert_eq!(query.condition_count(), 1);
        assert!(!query.is_always());
        assert!(!query.is_never());

        let multi = parse_media_attribute(Some("(min-width: 300px) and (max-width: 600px)"));
        assert_eq!(multi.condition_count(), 2);
    }

    #[test]
    fn parses_screen_or_all_prefixed_conditions() {
        let query = parse_media_attribute(Some("screen and (min-width: 300px)"));
        assert_eq!(query.condition_count(), 1);

        let query_all = parse_media_attribute(Some("all and (max-width: 500px)"));
        assert_eq!(query_all.condition_count(), 1);

        let query_print = parse_media_attribute(Some("print and (min-width: 300px)"));
        assert!(query_print.is_never());
    }

    #[test]
    fn malformed_or_unsupported_queries_result_in_never() {
        assert!(parse_media_attribute(Some("not all")).is_never());
        assert!(parse_media_attribute(Some("(unknown-feature: 10px)")).is_never());
        assert!(parse_media_attribute(Some("(min-width: 300px) trailing")).is_never());
        assert!(parse_media_attribute(Some("screen trailing")).is_never());
    }
}
