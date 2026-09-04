//! The selector grammar of the v0.5 cut
//! (`docs/reports/IMPLEMENTACAO-DETALHADA-V0-5.md` §2.8).
//!
//! Everything outside the cut is **refused with a `CssError`**, never accepted
//! and ignored: `::before` / `::after`, namespaces (`svg|rect`), `:has()`, any
//! unknown pseudo-class, and every attribute matcher beyond `[attr]` and
//! `[attr=v]`. A selector list containing one invalid selector is invalid
//! whole (CSS Selectors L4 §3.1) — the caller drops the rule and records a
//! note, which is why a half-applied rule can never happen.
//!
//! A parser function reads *and* advances the cursor. That is the one place
//! this crate does not split command from query: re-deriving the position after
//! a pure query would mean tokenizing twice. The [`TokenStream`] itself keeps
//! the split (`peek` answers, `advance` moves), and every lookahead below is a
//! pure query over it.

use crate::domain::error::{CssError, CssStage, SourceSpan};
use crate::domain::identifier::Identifier;
use crate::domain::selector::{
    AttributeMatch, AttributeSelector, Combinator, ComplexSelector, CompoundSelector, NthFormula,
    PseudoClass, SelectorList, SelectorStep, TypeSelector,
};
use crate::infrastructure::parser::token::{Token, TokenStream};

/// Whether a construct was found where one was optional.
enum Progress {
    /// One more component was read; keep going.
    Consumed,
    /// Nothing further belongs to this construct.
    Done,
}

/// Parses the selector list in front of a `{`, leaving the cursor on the token
/// that ended it.
pub(crate) fn parse_selector_list(tokens: &mut TokenStream) -> Result<SelectorList, CssError> {
    let mut list = SelectorList::new();
    list.push(parse_complex(tokens)?);
    while tokens.peek() == Some(&Token::Comma) {
        tokens.advance();
        list.push(parse_complex(tokens)?);
    }
    Ok(list)
}

/// One complex selector: compounds joined by combinators.
fn parse_complex(tokens: &mut TokenStream) -> Result<ComplexSelector, CssError> {
    tokens.skip_whitespace();
    let mut steps: Vec<SelectorStep> = Vec::new();
    steps.push(SelectorStep::new(
        Combinator::Descendant,
        parse_compound(tokens)?,
    ));
    while let Some((combinator, width)) = combinator_at(tokens) {
        advance_by(tokens, width);
        steps.push(SelectorStep::new(combinator, parse_compound(tokens)?));
    }
    tokens.skip_whitespace();
    Ok(ComplexSelector::new(steps))
}

/// The combinator separating the compound just read from the next one, and how
/// many tokens it occupies (leading whitespace, symbol, trailing whitespace).
///
/// A pure query: it moves nothing, so the caller decides whether to commit.
fn combinator_at(tokens: &TokenStream) -> Option<(Combinator, usize)> {
    let leading = usize::from(tokens.peek().is_some_and(Token::is_whitespace));
    let Some(combinator) = explicit_combinator(tokens.peek_ahead(leading)) else {
        return descendant_at(tokens, leading);
    };
    let after_symbol = leading.saturating_add(1);
    let trailing = usize::from(
        tokens
            .peek_ahead(after_symbol)
            .is_some_and(Token::is_whitespace),
    );
    Some((combinator, after_symbol.saturating_add(trailing)))
}

const fn explicit_combinator(token: Option<&Token>) -> Option<Combinator> {
    match token {
        Some(Token::Delimiter('>')) => Some(Combinator::Child),
        Some(Token::Delimiter('+')) => Some(Combinator::NextSibling),
        Some(Token::Delimiter('~')) => Some(Combinator::SubsequentSibling),
        _ => None,
    }
}

/// Whitespace is a descendant combinator only when another compound follows it;
/// the whitespace before `,` or `{` ends the selector instead.
fn descendant_at(tokens: &TokenStream, leading: usize) -> Option<(Combinator, usize)> {
    if leading == 0 || !starts_compound(tokens.peek_ahead(leading)) {
        return None;
    }
    Some((Combinator::Descendant, leading))
}

const fn starts_compound(token: Option<&Token>) -> bool {
    matches!(
        token,
        Some(
            Token::Ident(_)
                | Token::Hash(_)
                | Token::Colon
                | Token::OpenBracket
                | Token::Delimiter('.' | '*')
        )
    )
}

fn advance_by(tokens: &mut TokenStream, steps: usize) {
    for _step in 0..steps {
        tokens.advance();
    }
}

/// Everything that must hold of one element.
fn parse_compound(tokens: &mut TokenStream) -> Result<CompoundSelector, CssError> {
    let span = tokens.peek_span();
    let mut compound = CompoundSelector::universal();
    let mut written = matches!(
        parse_type_selector(tokens, &mut compound),
        Progress::Consumed
    );
    while matches!(parse_component(tokens, &mut compound)?, Progress::Consumed) {
        written = true;
    }
    if !written {
        return Err(selector_error("expected a selector", span));
    }
    Ok(compound)
}

/// The leading `tag` or `*`, if one was written.
fn parse_type_selector(tokens: &mut TokenStream, compound: &mut CompoundSelector) -> Progress {
    match tokens.peek().cloned() {
        Some(Token::Ident(name)) => set_named_type(tokens, compound, &name),
        Some(Token::Delimiter('*')) => set_universal_type(tokens, compound),
        _ => Progress::Done,
    }
}

fn set_named_type(
    tokens: &mut TokenStream,
    compound: &mut CompoundSelector,
    name: &str,
) -> Progress {
    let Some(identifier) = Identifier::lowercased(name) else {
        return Progress::Done;
    };
    tokens.advance();
    compound.set_type_selector(TypeSelector::Named(identifier));
    Progress::Consumed
}

fn set_universal_type(tokens: &mut TokenStream, compound: &mut CompoundSelector) -> Progress {
    tokens.advance();
    compound.set_type_selector(TypeSelector::Universal);
    Progress::Consumed
}

/// One `#id`, `.class`, `[attr…]` or `:pseudo-class`.
fn parse_component(
    tokens: &mut TokenStream,
    compound: &mut CompoundSelector,
) -> Result<Progress, CssError> {
    let span = tokens.peek_span();
    match tokens.peek().cloned() {
        Some(Token::Hash(name)) => push_id(tokens, compound, &name, span),
        Some(Token::Delimiter('.')) => push_class(tokens, compound, span),
        Some(Token::OpenBracket) => push_attribute(tokens, compound),
        Some(Token::Colon) => push_pseudo_class(tokens, compound),
        Some(Token::Delimiter('|')) => Err(selector_error(
            "namespace selectors are outside the v0.5 cut",
            span,
        )),
        _ => Ok(Progress::Done),
    }
}

fn push_id(
    tokens: &mut TokenStream,
    compound: &mut CompoundSelector,
    name: &str,
    span: SourceSpan,
) -> Result<Progress, CssError> {
    let identifier = identifier_or_error(name, span)?;
    tokens.advance();
    compound.push_id(identifier);
    Ok(Progress::Consumed)
}

fn push_class(
    tokens: &mut TokenStream,
    compound: &mut CompoundSelector,
    span: SourceSpan,
) -> Result<Progress, CssError> {
    tokens.advance();
    let Some(Token::Ident(name)) = tokens.peek().cloned() else {
        return Err(selector_error("`.` must be followed by a class name", span));
    };
    let identifier = identifier_or_error(&name, span)?;
    tokens.advance();
    compound.push_class(identifier);
    Ok(Progress::Consumed)
}

/// `[attr]` and `[attr=value]` — the two matchers of the cut.
fn push_attribute(
    tokens: &mut TokenStream,
    compound: &mut CompoundSelector,
) -> Result<Progress, CssError> {
    let span = tokens.peek_span();
    tokens.advance();
    tokens.skip_whitespace();
    let name = attribute_name(tokens, span)?;
    tokens.skip_whitespace();
    let match_kind = attribute_match(tokens, span)?;
    compound.push_attribute(AttributeSelector::new(name, match_kind));
    Ok(Progress::Consumed)
}

fn attribute_name(tokens: &mut TokenStream, span: SourceSpan) -> Result<Identifier, CssError> {
    let Some(Token::Ident(name)) = tokens.peek().cloned() else {
        return Err(selector_error(
            "`[` must be followed by an attribute name",
            span,
        ));
    };
    tokens.advance();
    Identifier::lowercased(&name).ok_or_else(|| selector_error("not a valid attribute name", span))
}

fn attribute_match(tokens: &mut TokenStream, span: SourceSpan) -> Result<AttributeMatch, CssError> {
    match tokens.peek().cloned() {
        Some(Token::CloseBracket) => Ok(close_existence_match(tokens)),
        Some(Token::Delimiter('=')) => close_exact_match(tokens, span),
        _ => Err(selector_error(
            "only `[attr]` and `[attr=value]` are supported in v0.5",
            span,
        )),
    }
}

const fn close_existence_match(tokens: &mut TokenStream) -> AttributeMatch {
    tokens.advance();
    AttributeMatch::Exists
}

fn close_exact_match(
    tokens: &mut TokenStream,
    span: SourceSpan,
) -> Result<AttributeMatch, CssError> {
    tokens.advance();
    tokens.skip_whitespace();
    let value = attribute_value(tokens, span)?;
    tokens.skip_whitespace();
    if tokens.peek() != Some(&Token::CloseBracket) {
        return Err(selector_error("unterminated attribute selector", span));
    }
    tokens.advance();
    Ok(AttributeMatch::Exact(value))
}

fn attribute_value(tokens: &mut TokenStream, span: SourceSpan) -> Result<String, CssError> {
    let Some(Token::Ident(value) | Token::QuotedString(value)) = tokens.peek().cloned() else {
        return Err(selector_error("`=` must be followed by a value", span));
    };
    tokens.advance();
    Ok(value)
}

/// `:hover`, `:first-child`, `:nth-child(…)` — and a refusal for `::` and for
/// every pseudo-class outside the cut.
fn push_pseudo_class(
    tokens: &mut TokenStream,
    compound: &mut CompoundSelector,
) -> Result<Progress, CssError> {
    let span = tokens.peek_span();
    tokens.advance();
    if tokens.peek() == Some(&Token::Colon) {
        return Err(selector_error(
            "pseudo-elements (`::before` / `::after`) are outside the v0.5 cut",
            span,
        ));
    }
    let pseudo_class = pseudo_class_at(tokens, span)?;
    compound.push_pseudo_class(pseudo_class);
    Ok(Progress::Consumed)
}

fn pseudo_class_at(tokens: &mut TokenStream, span: SourceSpan) -> Result<PseudoClass, CssError> {
    match tokens.peek().cloned() {
        Some(Token::Ident(name)) => plain_pseudo_class(tokens, &name, span),
        Some(Token::Function(name)) => functional_pseudo_class(tokens, &name, span),
        _ => Err(selector_error(
            "`:` must be followed by a pseudo-class",
            span,
        )),
    }
}

fn plain_pseudo_class(
    tokens: &mut TokenStream,
    name: &str,
    span: SourceSpan,
) -> Result<PseudoClass, CssError> {
    let pseudo_class = match name.to_ascii_lowercase().as_str() {
        "hover" => PseudoClass::Hover,
        "active" => PseudoClass::Active,
        "focus" => PseudoClass::Focus,
        "first-child" => PseudoClass::FirstChild,
        "last-child" => PseudoClass::LastChild,
        _ => return Err(unsupported_pseudo_class(name, span)),
    };
    tokens.advance();
    Ok(pseudo_class)
}

fn functional_pseudo_class(
    tokens: &mut TokenStream,
    name: &str,
    span: SourceSpan,
) -> Result<PseudoClass, CssError> {
    if !name.eq_ignore_ascii_case("nth-child") {
        return Err(unsupported_pseudo_class(name, span));
    }
    tokens.advance();
    let arguments = function_arguments(tokens, span)?;
    Ok(PseudoClass::NthChild(parse_nth_formula(&arguments, span)?))
}

fn unsupported_pseudo_class(name: &str, span: SourceSpan) -> CssError {
    selector_error(
        format!("`:{name}` is outside the v0.5 cut (see tests/data/MANIFEST.md)"),
        span,
    )
}

/// The non-whitespace tokens up to the `)` that closes a function.
fn function_arguments(tokens: &mut TokenStream, span: SourceSpan) -> Result<Vec<Token>, CssError> {
    let mut arguments: Vec<Token> = Vec::new();
    while let Some(token) = tokens.peek().cloned() {
        tokens.advance();
        if token == Token::CloseParenthesis {
            return Ok(arguments);
        }
        push_argument(&mut arguments, token);
    }
    Err(selector_error("unterminated pseudo-class argument", span))
}

fn push_argument(arguments: &mut Vec<Token>, token: Token) {
    if token.is_whitespace() {
        return;
    }
    arguments.push(token);
}

/// `an+b`, `odd`, `even`, a bare integer, `n`, `-n+3` (CSS Selectors L4 §6.6.3).
fn parse_nth_formula(arguments: &[Token], span: SourceSpan) -> Result<NthFormula, CssError> {
    let terms = strip_leading_plus(arguments);
    let (first, rest) = terms
        .split_first()
        .ok_or_else(|| selector_error("`:nth-child()` needs an argument", span))?;
    match first {
        Token::Ident(text) => nth_from_identifier(text, rest, span),
        Token::Dimension(value, unit) => nth_from_dimension(*value, unit, rest, span),
        Token::Number(value) => nth_constant(*value, rest, span),
        _ => Err(selector_error(
            "`:nth-child()` argument is not `an+b`",
            span,
        )),
    }
}

/// `:nth-child(+2n+1)` — a leading `+` that the tokenizer kept as a delimiter
/// because a letter, not a digit, followed it.
const fn strip_leading_plus(arguments: &[Token]) -> &[Token] {
    let Some((Token::Delimiter('+'), rest)) = arguments.split_first() else {
        return arguments;
    };
    rest
}

fn nth_from_identifier(
    text: &str,
    rest: &[Token],
    span: SourceSpan,
) -> Result<NthFormula, CssError> {
    let lowered = text.to_ascii_lowercase();
    if lowered == "odd" {
        return finish_keyword(NthFormula::new(2, 1), rest, span);
    }
    if lowered == "even" {
        return finish_keyword(NthFormula::new(2, 0), rest, span);
    }
    let (step, tail) = signed_step(&lowered).ok_or_else(|| not_an_nth(span))?;
    nth_with_offset(step, tail, rest, span)
}

/// `n` and `-n` carry an implicit `a` of `1` and `-1`; anything after the `n`
/// is the `-b` half of `n-3`.
fn signed_step(lowered: &str) -> Option<(i32, &str)> {
    let positive = lowered.strip_prefix('n').map(|tail| (1, tail));
    let negative = lowered.strip_prefix("-n").map(|tail| (-1, tail));
    positive.or(negative)
}

fn nth_from_dimension(
    value: f32,
    unit: &str,
    rest: &[Token],
    span: SourceSpan,
) -> Result<NthFormula, CssError> {
    let lowered = unit.to_ascii_lowercase();
    let tail = lowered.strip_prefix('n').ok_or_else(|| not_an_nth(span))?;
    let step = integer_value(value).ok_or_else(|| not_an_nth(span))?;
    nth_with_offset(step, tail, rest, span)
}

/// A bare `:nth-child(3)` is `0n+3`.
fn nth_constant(value: f32, rest: &[Token], span: SourceSpan) -> Result<NthFormula, CssError> {
    let offset = integer_value(value).ok_or_else(|| not_an_nth(span))?;
    finish_keyword(NthFormula::new(0, offset), rest, span)
}

fn finish_keyword(
    formula: NthFormula,
    rest: &[Token],
    span: SourceSpan,
) -> Result<NthFormula, CssError> {
    if !rest.is_empty() {
        return Err(not_an_nth(span));
    }
    Ok(formula)
}

/// The `b` half: glued to the unit (`2n-1`), a signed number (`2n+1`), or a
/// sign and a number separated by whitespace (`2n + 1`).
fn nth_with_offset(
    step: i32,
    tail: &str,
    rest: &[Token],
    span: SourceSpan,
) -> Result<NthFormula, CssError> {
    if !tail.is_empty() {
        let offset = tail.parse::<i32>().map_err(|_error| not_an_nth(span))?;
        return finish_keyword(NthFormula::new(step, offset), rest, span);
    }
    Ok(NthFormula::new(step, trailing_offset(rest, span)?))
}

fn trailing_offset(rest: &[Token], span: SourceSpan) -> Result<i32, CssError> {
    match rest {
        [] => Ok(0),
        [Token::Number(value)] => integer_value(*value).ok_or_else(|| not_an_nth(span)),
        [Token::Delimiter(sign), Token::Number(value)] => signed_offset(*sign, *value, span),
        _ => Err(not_an_nth(span)),
    }
}

fn signed_offset(sign: char, value: f32, span: SourceSpan) -> Result<i32, CssError> {
    let magnitude = integer_value(value).ok_or_else(|| not_an_nth(span))?;
    match sign {
        '+' => Ok(magnitude),
        '-' => magnitude.checked_neg().ok_or_else(|| not_an_nth(span)),
        _ => Err(not_an_nth(span)),
    }
}

/// The integer a numeric token stands for, or `None` when it is not one.
///
/// `as` casts are denied (`Cargo.toml:82`) and there is no `TryFrom<f32>` for
/// `i32`, so the value is round-tripped through its decimal text: `2.0` prints
/// as `2` and parses as `2`, while `1.5` prints as `1.5` and fails to parse —
/// which is exactly the acceptance rule `an+b` needs.
fn integer_value(value: f32) -> Option<i32> {
    format!("{value}").parse::<i32>().ok()
}

fn not_an_nth(span: SourceSpan) -> CssError {
    selector_error("`:nth-child()` argument is not `an+b`", span)
}

fn identifier_or_error(name: &str, span: SourceSpan) -> Result<Identifier, CssError> {
    Identifier::new(name).ok_or_else(|| selector_error("not a valid identifier", span))
}

fn selector_error(detail: impl Into<String>, span: SourceSpan) -> CssError {
    CssError::unsupported(CssStage::Selector, detail).with_span(span)
}
