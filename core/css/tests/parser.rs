//! Guards the CSS Syntax Level 3 tokenizer and rule parser: that tokenizing is
//! **total** (no input fails, no input loops), that recovery follows §5.4, and
//! that every recovery leaves a `ParseNote` instead of shrinking the declared
//! cut in silence (`relatório §2.8:350-354`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use css::{
    Origin, ParseNotes, StyleSheetSet, Token, parse_inline_style, parse_stylesheet, tokenize,
};

/// Every token of `source`, with whitespace runs dropped — what a grammar
/// assertion cares about.
fn significant_tokens(source: &str) -> Vec<Token> {
    tokenize(source)
        .iter()
        .map(|spanned| spanned.token().clone())
        .filter(|token| !token.is_whitespace())
        .collect()
}

fn author_sheet(source: &str) -> StyleSheetSet {
    parse_stylesheet(source, Origin::Author).expect("the source is not hostile")
}

fn note_messages(notes: &ParseNotes) -> Vec<String> {
    notes.iter().map(|note| note.message().to_owned()).collect()
}

fn assert_note_mentions(sheets: &StyleSheetSet, fragment: &str) {
    let messages = note_messages(sheets.notes());
    assert!(
        messages.iter().any(|message| message.contains(fragment)),
        "expected a note mentioning `{fragment}`, got {messages:?}"
    );
}

// ---- tokenizer: comments, strings, escapes, url(), numbers ----------------

#[test]
fn comments_produce_no_tokens_and_an_unterminated_one_eats_the_rest() {
    assert_eq!(
        significant_tokens("a /* gone */ b"),
        vec![Token::Ident("a".to_owned()), Token::Ident("b".to_owned())]
    );
    assert_eq!(
        significant_tokens("a /* never closed"),
        vec![Token::Ident("a".to_owned())],
        "an unterminated comment runs to the end of the source (§4.3.2)"
    );
}

#[test]
fn markup_comment_delimiters_are_skipped_at_the_top_level() {
    assert_eq!(
        significant_tokens("<!-- p -->"),
        vec![Token::Ident("p".to_owned())],
        "old sheets wrap themselves in `<!--` / `-->` (§5.3.3)"
    );
}

#[test]
fn strings_take_both_quote_forms_and_resolve_escapes() {
    assert_eq!(
        significant_tokens("'single' \"double\""),
        vec![
            Token::QuotedString("single".to_owned()),
            Token::QuotedString("double".to_owned())
        ]
    );
    assert_eq!(
        significant_tokens(r#""\41\42""#),
        vec![Token::QuotedString("AB".to_owned())],
        "`\\41` is a hex escape for `A` (§4.3.7)"
    );
    assert_eq!(
        significant_tokens(r#""\41 B""#),
        vec![Token::QuotedString("AB".to_owned())],
        "one whitespace after a hex escape terminates it and is consumed"
    );
    assert_eq!(
        significant_tokens(r#""a\"b""#),
        vec![Token::QuotedString("a\"b".to_owned())],
        "a non-hex escape is the literal character"
    );
}

#[test]
fn an_unterminated_string_becomes_a_bad_string_rather_than_a_failure() {
    assert_eq!(significant_tokens("\"open"), vec![Token::BadString]);
    assert_eq!(
        significant_tokens("\"open\nclosed\""),
        vec![
            Token::BadString,
            Token::Ident("closed".to_owned()),
            Token::BadString
        ],
        "a newline ends a string (§4.3.5); the rest still tokenizes"
    );
}

#[test]
fn an_escape_survives_inside_an_identifier_and_at_end_of_source() {
    assert_eq!(
        significant_tokens(r"\41 lloy"),
        vec![Token::Ident("Alloy".to_owned())]
    );
    assert_eq!(
        significant_tokens("a\\"),
        vec![Token::Ident("a\u{FFFD}".to_owned())],
        "a trailing `\\` resolves to U+FFFD (§4.3.7)"
    );
}

#[test]
fn url_is_a_url_token_unquoted_and_a_function_token_quoted() {
    assert_eq!(
        significant_tokens("url(a/b.png)"),
        vec![Token::Url("a/b.png".to_owned())]
    );
    assert_eq!(
        significant_tokens("url(  spaced  )"),
        vec![Token::Url("spaced".to_owned())],
        "whitespace inside `url()` is trimmed, not part of the payload"
    );
    assert_eq!(
        significant_tokens("url(\"quoted\")"),
        vec![
            Token::Function("url".to_owned()),
            Token::QuotedString("quoted".to_owned()),
            Token::CloseParenthesis
        ],
        "only an unquoted payload is a url token (§4.3.4)"
    );
    assert_eq!(significant_tokens("url(never-closed"), vec![Token::BadUrl]);
    assert_eq!(
        significant_tokens("url(a b) p"),
        vec![Token::BadUrl, Token::Ident("p".to_owned())],
        "a bad url is consumed up to its `)` so the rest still tokenizes"
    );
}

#[test]
fn numbers_dimensions_and_percentages_are_distinguished() {
    assert_eq!(
        significant_tokens("0 1.5 -3 +2 1e2 -1.5e-2"),
        vec![
            Token::Number(0.0),
            Token::Number(1.5),
            Token::Number(-3.0),
            Token::Number(2.0),
            Token::Number(100.0),
            Token::Number(-0.015),
        ]
    );
    assert_eq!(
        significant_tokens("16px 1.2em 50%"),
        vec![
            Token::Dimension(16.0, "px".to_owned()),
            Token::Dimension(1.2, "em".to_owned()),
            Token::Percentage(50.0),
        ]
    );
    assert_eq!(
        significant_tokens(".5px"),
        vec![Token::Dimension(0.5, "px".to_owned())],
        "a leading `.` starts a number when a digit follows"
    );
}

#[test]
fn structural_punctuation_and_leftovers_are_distinct_tokens() {
    assert_eq!(
        significant_tokens("#main @media fn( > ~ !"),
        vec![
            Token::Hash("main".to_owned()),
            Token::AtKeyword("media".to_owned()),
            Token::Function("fn".to_owned()),
            Token::Delimiter('>'),
            Token::Delimiter('~'),
            Token::Delimiter('!'),
        ]
    );
}

#[test]
fn a_token_carries_the_one_based_line_and_column_it_started_at() {
    let stream = tokenize("p\n  q");
    let spans: Vec<(u32, u32)> = stream
        .iter()
        .filter(|spanned| !spanned.token().is_whitespace())
        .map(|spanned| (spanned.span().line(), spanned.span().column()))
        .collect();
    assert_eq!(spans, vec![(1, 1), (2, 3)]);
}

#[test]
fn carriage_returns_and_nulls_are_preprocessed_away() {
    let stream = tokenize("a\r\nb\0");
    let idents: Vec<Token> = stream
        .iter()
        .map(|spanned| spanned.token().clone())
        .filter(|token| !token.is_whitespace())
        .collect();
    assert_eq!(
        idents,
        vec![
            Token::Ident("a".to_owned()),
            Token::Ident("b\u{FFFD}".to_owned())
        ],
        "§3.3 collapses newline forms and replaces NULL"
    );
}

// ---- rule parser: rules, declarations, @media -----------------------------

#[test]
fn a_qualified_rule_becomes_one_selector_list_and_one_declaration_block() {
    let sheets = author_sheet("p, .lead { color: red; margin: 4px 8px }");
    assert_eq!(sheets.len(), 1);
    let (origin, rule) = sheets.rules().next().expect("one rule");

    assert_eq!(origin, Origin::Author);
    assert_eq!(rule.selectors().to_string(), "p, .lead");
    assert_eq!(rule.declarations().len(), 2);
    assert_eq!(
        rule.declarations()
            .last_of("margin")
            .map(|declaration| declaration.value().to_string()),
        Some("4px 8px".to_owned())
    );
    assert!(sheets.notes().is_empty(), "a clean sheet raises no note");
}

#[test]
fn important_is_preserved_rather_than_dropped() {
    let sheets = author_sheet("p { color: red !important }");
    let (_, rule) = sheets.rules().next().expect("one rule");
    let declaration = rule.declarations().last_of("color").expect("the colour");

    assert!(declaration.importance().is_important());
    assert_eq!(
        declaration.value().as_str(),
        "red",
        "`!important` is stripped from the value it qualifies"
    );
}

#[test]
fn a_media_block_gates_its_rules_and_the_producer_discharges_the_condition() {
    let sheets =
        author_sheet("@media (min-width: 600px) and (max-width: 900px) { p { color: red } }");
    assert_eq!(sheets.len(), 1);
    let (_, rule) = sheets.rules().next().expect("one rule");
    assert_eq!(rule.media().condition_count(), 2);
    assert!(!rule.media().is_always());

    let wide = css::ViewportConstraints::new(whole_px(800), whole_px(600));
    let narrow = css::ViewportConstraints::new(whole_px(320), whole_px(600));
    assert_eq!(sheets.matching_viewport(&wide).len(), 1);
    assert_eq!(sheets.matching_viewport(&narrow).len(), 0);
    assert!(
        sheets
            .matching_viewport(&wide)
            .rules()
            .all(|(_, kept)| kept.media().is_always()),
        "a surviving rule is rewritten unconditional so the resolver can apply it"
    );
}

const fn whole_px(pixels: i32) -> graphics::Au {
    graphics::Au::from_whole_px(pixels).expect("a small pixel count fits")
}

#[test]
fn rules_outside_and_inside_a_media_block_keep_their_source_order() {
    let sheets = author_sheet(
        "a { color: red } @media (min-width: 1px) { b { color: blue } } i { color: lime }",
    );
    let selectors: Vec<String> = sheets
        .rules()
        .map(|(_, rule)| rule.selectors().to_string())
        .collect();
    assert_eq!(selectors, vec!["a", "b", "i"]);
}

// ---- recovery: everything outside the cut is recorded, never silent -------

#[test]
fn an_unknown_at_rule_is_skipped_with_a_note_and_costs_no_later_rule() {
    for source in [
        "@supports (display: flex) { p { color: red } } q { color: blue }",
        "@font-face { font-family: x } q { color: blue }",
        "@keyframes spin { from { color: red } } q { color: blue }",
    ] {
        let sheets = author_sheet(source);
        assert_eq!(sheets.len(), 1, "only `q` survives `{source}`");
        assert_note_mentions(&sheets, "outside the v0.5 cut");
    }
}

#[test]
fn an_at_rule_with_no_block_is_skipped_at_its_semicolon() {
    let sheets = author_sheet("@import url(other.css); q { color: blue }");
    assert_eq!(sheets.len(), 1);
    assert_note_mentions(&sheets, "`@import` is outside the v0.5 cut");
}

#[test]
fn a_selector_outside_the_cut_drops_its_whole_rule_with_a_note() {
    for (source, fragment) in [
        (":has(p) { color: red } q { color: blue }", "`:has`"),
        (
            "p::before { color: red } q { color: blue }",
            "pseudo-element",
        ),
        (
            "p:nonsense { color: red } q { color: blue }",
            "outside the v0.5 cut",
        ),
        (
            "[href^=\"x\"] { color: red } q { color: blue }",
            "only `[attr]`",
        ),
    ] {
        let sheets = author_sheet(source);
        assert_eq!(sheets.len(), 1, "only `q` survives `{source}`");
        assert_note_mentions(&sheets, fragment);
    }
}

#[test]
fn a_namespace_selector_is_refused_rather_than_read_as_a_type() {
    let sheets = author_sheet("svg|rect { color: red } q { color: blue }");
    assert_eq!(sheets.len(), 1);
    assert_note_mentions(&sheets, "namespace");
}

#[test]
fn an_unsupported_property_drops_that_declaration_only() {
    let sheets = author_sheet("p { float: left; color: red }");
    let (_, rule) = sheets.rules().next().expect("the rule survives");

    assert_eq!(rule.declarations().len(), 1, "only `float` is dropped");
    assert!(rule.declarations().last_of("color").is_some());
    assert_note_mentions(&sheets, "`float` is outside the v0.5 property cut");
}

#[test]
fn a_declaration_with_no_colon_is_recovered_at_the_next_semicolon() {
    let sheets = author_sheet("p { color red; margin: 4px }");
    let (_, rule) = sheets.rules().next().expect("the rule survives");

    assert_eq!(rule.declarations().len(), 1);
    assert!(rule.declarations().last_of("margin").is_some());
    assert_note_mentions(&sheets, "has no `:` value");
}

#[test]
fn a_stray_close_brace_is_noted_and_the_next_rule_still_parses() {
    let sheets = author_sheet("} p { color: red }");
    assert_eq!(sheets.len(), 1);
    assert_note_mentions(&sheets, "stray `}`");
}

#[test]
fn an_unterminated_rule_block_ends_at_the_source_and_still_yields_its_rule() {
    let sheets = author_sheet("p { color: red");
    assert_eq!(sheets.len(), 1);
    assert_eq!(
        sheets
            .rules()
            .next()
            .and_then(|(_, rule)| rule.declarations().last_of("color"))
            .map(|declaration| declaration.value().to_string()),
        Some("red".to_owned())
    );
}

#[test]
fn an_unterminated_media_block_is_noted_and_keeps_the_rules_it_held() {
    let sheets = author_sheet("@media (min-width: 1px) { p { color: red }");
    assert_eq!(sheets.len(), 1);
    assert_note_mentions(&sheets, "unterminated `@media` block");
}

#[test]
fn a_media_prelude_outside_the_cut_skips_the_block_with_a_note() {
    let sheets = author_sheet("@media print { p { color: red } } q { color: blue }");
    assert_eq!(sheets.len(), 1);
    assert_note_mentions(&sheets, "media condition must be parenthesised");
}

#[test]
fn an_unreadable_value_is_noted_when_it_carries_a_bad_string() {
    let sheets = author_sheet("p { color: \"open }");
    assert_note_mentions(&sheets, "unterminated string or url");
}

#[test]
fn hostile_nesting_is_the_one_failure_that_escapes_as_an_error() {
    let source = format!("@x {}{}", "{".repeat(40), "}".repeat(40));
    let error = parse_stylesheet(&source, Origin::Author).expect_err("40 levels is refused");

    assert_eq!(error.stage(), css::CssStage::Parse);
    assert!(error.span().is_some(), "the refusal carries a location");
    assert!(error.to_string().contains("nesting"));
}

#[test]
fn a_source_that_is_only_noise_parses_to_an_empty_sheet_without_looping() {
    for source in ["", "   ", "/* only a comment */", ";;;", "&&&", "{{{}}}"] {
        let sheets = author_sheet(source);
        assert!(sheets.is_empty(), "`{source}` yields no rule");
    }
}

// ---- inline style attributes ---------------------------------------------

#[test]
fn an_inline_block_parses_without_selector_or_braces() {
    let block = parse_inline_style("color: #f00; margin-top: 4px").expect("a readable block");
    assert_eq!(block.len(), 2);
    assert_eq!(
        block
            .last_of("margin-top")
            .map(|declaration| declaration.value().to_string()),
        Some("4px".to_owned())
    );
}

#[test]
fn an_inline_block_drops_an_unsupported_property_and_keeps_the_rest() {
    let block = parse_inline_style("float: left; color: red").expect("a readable block");
    assert_eq!(block.len(), 1);
    assert!(block.last_of("color").is_some());
}
