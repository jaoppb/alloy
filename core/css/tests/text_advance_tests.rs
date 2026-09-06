//! Unit and integration tests for CSS Advanced Typography & Line Formatting.
//!
//! Covers:
//! 1. `font-weight` (normal, bold, numeric 100..900, bolder, lighter)
//! 2. `font-style` (normal, italic, oblique)
//! 3. `line-height` (normal, Length, number unitless, percentage)
//! 4. `letter-spacing`, `word-spacing` (normal, Length)
//! 5. `text-decoration-line`, `text-decoration-color`, `text-decoration-style`, shorthand `text-decoration`
//! 6. `text-transform` (none, capitalize, uppercase, lowercase)
//! 7. `text-overflow` (clip, ellipsis)
//! 8. `overflow-wrap` (normal, break-word, anywhere) and `word-wrap` alias
//! 9. `word-break` (normal, break-all, keep-all)
//! 10. `TextAdvanceStyle` aggregate, builders, initial values, and typographical inheritance.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use css::domain::color::CssColor;
use css::domain::computed::text_advance::{
    FontStyle, FontWeight, LetterSpacing, LineHeight, LineHeightFactor, LineHeightPercentage,
    OverflowWrap, TextAdvanceStyle, TextDecoration, TextDecorationLine, TextDecorationStyle,
    TextOverflow, TextTransform, WordBreak, WordSpacing,
};
use css::domain::length::Length;
use css::infrastructure::cascade::text_values::{
    apply, inherit, parse_font_style, parse_font_weight, parse_font_weight_with_parent,
    parse_letter_spacing, parse_line_height, parse_overflow_wrap, parse_text_decoration,
    parse_text_decoration_color, parse_text_decoration_line, parse_text_decoration_style,
    parse_text_overflow, parse_text_transform, parse_word_break, parse_word_spacing, reset,
};
use css::infrastructure::parser::token::Token;
use graphics::Au;

const fn au(pixels: i32) -> Au {
    match Au::from_whole_px(pixels) {
        Some(val) => val,
        None => Au::ZERO,
    }
}

// -----------------------------------------------------------------------------
// 1. font-weight
// -----------------------------------------------------------------------------

#[test]
fn font_weight_constants_and_constructors() {
    assert_eq!(FontWeight::NORMAL.value(), 400);
    assert_eq!(FontWeight::BOLD.value(), 700);
    assert_eq!(FontWeight::THIN.value(), 100);
    assert_eq!(FontWeight::EXTRA_LIGHT.value(), 200);
    assert_eq!(FontWeight::LIGHT.value(), 300);
    assert_eq!(FontWeight::MEDIUM.value(), 500);
    assert_eq!(FontWeight::SEMI_BOLD.value(), 600);
    assert_eq!(FontWeight::EXTRA_BOLD.value(), 800);
    assert_eq!(FontWeight::BLACK.value(), 900);

    assert_eq!(FontWeight::new(400), Some(FontWeight::NORMAL));
    assert_eq!(FontWeight::new(700), Some(FontWeight::BOLD));
    assert_eq!(FontWeight::new(0), None);
    assert_eq!(FontWeight::new(1001), None);
}

#[test]
fn font_weight_bolder_and_lighter_rules() {
    // CSS Fonts 4 §2.4 relative weights:
    // < 400 -> 400
    assert_eq!(FontWeight::THIN.bolder(), FontWeight::NORMAL);
    assert_eq!(FontWeight::LIGHT.bolder(), FontWeight::NORMAL);
    // 400..=500 -> 700
    assert_eq!(FontWeight::NORMAL.bolder(), FontWeight::BOLD);
    assert_eq!(FontWeight::MEDIUM.bolder(), FontWeight::BOLD);
    // > 500 -> 900
    assert_eq!(FontWeight::SEMI_BOLD.bolder(), FontWeight::BLACK);
    assert_eq!(FontWeight::BOLD.bolder(), FontWeight::BLACK);
    assert_eq!(FontWeight::BLACK.bolder(), FontWeight::BLACK);

    // lighter:
    // < 600 -> 100
    assert_eq!(FontWeight::NORMAL.lighter(), FontWeight::THIN);
    assert_eq!(FontWeight::MEDIUM.lighter(), FontWeight::THIN);
    // 600..=700 -> 400
    assert_eq!(FontWeight::SEMI_BOLD.lighter(), FontWeight::NORMAL);
    assert_eq!(FontWeight::BOLD.lighter(), FontWeight::NORMAL);
    // > 700 -> 700
    assert_eq!(FontWeight::EXTRA_BOLD.lighter(), FontWeight::BOLD);
    assert_eq!(FontWeight::BLACK.lighter(), FontWeight::BOLD);
}

#[test]
fn font_weight_is_bold_and_display() {
    assert!(!FontWeight::NORMAL.is_bold());
    assert!(!FontWeight::MEDIUM.is_bold());
    assert!(FontWeight::BOLD.is_bold());
    assert!(FontWeight::BLACK.is_bold());

    assert_eq!(format!("{}", FontWeight::NORMAL), "normal");
    assert_eq!(format!("{}", FontWeight::BOLD), "bold");
    assert_eq!(format!("{}", FontWeight::THIN), "100");
}

#[test]
fn font_weight_parsing() {
    assert_eq!(
        parse_font_weight(&[Token::Ident("normal".into())]),
        Some(FontWeight::NORMAL)
    );
    assert_eq!(
        parse_font_weight(&[Token::Ident("bold".into())]),
        Some(FontWeight::BOLD)
    );
    assert_eq!(
        parse_font_weight(&[Token::Number(300.0)]),
        Some(FontWeight::LIGHT)
    );
    assert_eq!(
        parse_font_weight(&[Token::Number(700.0)]),
        Some(FontWeight::BOLD)
    );
    assert_eq!(parse_font_weight(&[Token::Number(1200.0)]), None);
    assert_eq!(parse_font_weight(&[Token::Ident("italic".into())]), None);

    // relative parsing with parent
    assert_eq!(
        parse_font_weight_with_parent(&[Token::Ident("bolder".into())], FontWeight::NORMAL),
        Some(FontWeight::BOLD)
    );
    assert_eq!(
        parse_font_weight_with_parent(&[Token::Ident("lighter".into())], FontWeight::BOLD),
        Some(FontWeight::NORMAL)
    );
}

// -----------------------------------------------------------------------------
// 2. font-style
// -----------------------------------------------------------------------------

#[test]
fn font_style_variants_and_parsing() {
    assert_eq!(FontStyle::default(), FontStyle::Normal);
    assert!(FontStyle::Italic.is_italic());
    assert!(!FontStyle::Normal.is_italic());
    assert!(FontStyle::Oblique.is_oblique());

    assert_eq!(FontStyle::Normal.keyword(), "normal");
    assert_eq!(FontStyle::Italic.keyword(), "italic");
    assert_eq!(FontStyle::Oblique.keyword(), "oblique");

    assert_eq!(
        parse_font_style(&[Token::Ident("normal".into())]),
        Some(FontStyle::Normal)
    );
    assert_eq!(
        parse_font_style(&[Token::Ident("italic".into())]),
        Some(FontStyle::Italic)
    );
    assert_eq!(
        parse_font_style(&[Token::Ident("oblique".into())]),
        Some(FontStyle::Oblique)
    );
    assert_eq!(parse_font_style(&[Token::Ident("bold".into())]), None);
}

// -----------------------------------------------------------------------------
// 3. line-height
// -----------------------------------------------------------------------------

#[test]
fn line_height_variants_resolution_and_parsing() {
    let font_size = au(16);

    let normal = LineHeight::Normal;
    assert!(normal.is_normal());
    assert_eq!(
        normal.resolve_to_au(font_size),
        graphics::Au::from_px(graphics::Px::new(19.2))
    );

    let length = LineHeight::Length(Length::Pixels(24.0));
    assert_eq!(length.resolve_to_au(font_size), Some(au(24)));

    let number = LineHeight::Number(LineHeightFactor::new(1.5));
    assert_eq!(number.resolve_to_au(font_size), Some(au(24))); // 16 * 1.5 = 24

    let percent = LineHeight::Percentage(LineHeightPercentage::new(150.0));
    assert_eq!(percent.resolve_to_au(font_size), Some(au(24))); // 16 * 1.5 = 24

    // Parsing
    assert_eq!(
        parse_line_height(&[Token::Ident("normal".into())]),
        Some(LineHeight::Normal)
    );
    assert_eq!(
        parse_line_height(&[Token::Dimension(20.0, "px".into())]),
        Some(LineHeight::Length(Length::Pixels(20.0)))
    );
    assert_eq!(
        parse_line_height(&[Token::Number(1.5)]),
        Some(LineHeight::Number(LineHeightFactor::new(1.5)))
    );
    assert_eq!(
        parse_line_height(&[Token::Number(0.0)]),
        Some(LineHeight::Length(Length::ZERO))
    );
    assert_eq!(
        parse_line_height(&[Token::Percentage(120.0)]),
        Some(LineHeight::Percentage(LineHeightPercentage::new(120.0)))
    );
    assert_eq!(parse_line_height(&[Token::Number(-1.0)]), None);
}

// -----------------------------------------------------------------------------
// 4. letter-spacing and word-spacing
// -----------------------------------------------------------------------------

#[test]
fn letter_and_word_spacing() {
    let font_size = au(16);

    let normal_letter = LetterSpacing::Normal;
    assert!(normal_letter.is_normal());
    assert_eq!(normal_letter.resolve_to_au(font_size), Some(Au::ZERO));

    let len_letter = LetterSpacing::Length(Length::Pixels(2.0));
    assert_eq!(len_letter.resolve_to_au(font_size), Some(au(2)));

    let normal_word = WordSpacing::Normal;
    assert!(normal_word.is_normal());
    assert_eq!(normal_word.resolve_to_au(font_size), Some(Au::ZERO));

    let len_word = WordSpacing::Length(Length::Pixels(4.0));
    assert_eq!(len_word.resolve_to_au(font_size), Some(au(4)));

    // Parsing
    assert_eq!(
        parse_letter_spacing(&[Token::Ident("normal".into())]),
        Some(LetterSpacing::Normal)
    );
    assert_eq!(
        parse_letter_spacing(&[Token::Dimension(3.0, "px".into())]),
        Some(LetterSpacing::Length(Length::Pixels(3.0)))
    );

    assert_eq!(
        parse_word_spacing(&[Token::Ident("normal".into())]),
        Some(WordSpacing::Normal)
    );
    assert_eq!(
        parse_word_spacing(&[Token::Dimension(5.0, "px".into())]),
        Some(WordSpacing::Length(Length::Pixels(5.0)))
    );
}

// -----------------------------------------------------------------------------
// 5. text-decoration (line, color, style, shorthand)
// -----------------------------------------------------------------------------

#[test]
fn text_decoration_line_builders_and_parsing() {
    let none = TextDecorationLine::NONE;
    assert!(none.is_none());
    assert_eq!(format!("{none}"), "none");

    let underline = TextDecorationLine::UNDERLINE;
    assert!(underline.has_underline());
    assert!(!underline.has_overline());
    assert_eq!(format!("{underline}"), "underline");

    let both = underline.with_line_through(true);
    assert!(both.has_underline());
    assert!(both.has_line_through());
    assert_eq!(format!("{both}"), "underline line-through");

    // Parsing
    assert_eq!(
        parse_text_decoration_line(&[Token::Ident("none".into())]),
        Some(TextDecorationLine::NONE)
    );
    assert_eq!(
        parse_text_decoration_line(&[Token::Ident("underline".into())]),
        Some(TextDecorationLine::UNDERLINE)
    );
    assert_eq!(
        parse_text_decoration_line(&[
            Token::Ident("underline".into()),
            Token::Ident("line-through".into())
        ]),
        Some(TextDecorationLine::UNDERLINE.with_line_through(true))
    );
    assert_eq!(
        parse_text_decoration_line(&[Token::Ident("invalid".into())]),
        None
    );
}

#[test]
fn text_decoration_style_and_color_parsing() {
    assert_eq!(
        parse_text_decoration_style(&[Token::Ident("solid".into())]),
        Some(TextDecorationStyle::Solid)
    );
    assert_eq!(
        parse_text_decoration_style(&[Token::Ident("double".into())]),
        Some(TextDecorationStyle::Double)
    );
    assert_eq!(
        parse_text_decoration_style(&[Token::Ident("dotted".into())]),
        Some(TextDecorationStyle::Dotted)
    );
    assert_eq!(
        parse_text_decoration_style(&[Token::Ident("dashed".into())]),
        Some(TextDecorationStyle::Dashed)
    );
    assert_eq!(
        parse_text_decoration_style(&[Token::Ident("wavy".into())]),
        Some(TextDecorationStyle::Wavy)
    );
    assert_eq!(
        parse_text_decoration_style(&[Token::Ident("unknown".into())]),
        None
    );

    assert_eq!(
        parse_text_decoration_color(&[Token::Ident("red".into())]),
        Some(CssColor::rgb(255, 0, 0))
    );
}

#[test]
fn text_decoration_shorthand_parsing() {
    assert_eq!(
        parse_text_decoration(&[Token::Ident("none".into())]),
        Some(TextDecoration::initial())
    );

    let red = CssColor::rgb(255, 0, 0);
    // underline red wavy
    let parsed = parse_text_decoration(&[
        Token::Ident("underline".into()),
        Token::Ident("red".into()),
        Token::Ident("wavy".into()),
    ]);
    assert_eq!(
        parsed,
        Some(
            TextDecoration::initial()
                .with_line(TextDecorationLine::UNDERLINE)
                .with_color(red)
                .with_style(TextDecorationStyle::Wavy)
        )
    );

    // dashed line-through
    let parsed_dashed = parse_text_decoration(&[
        Token::Ident("dashed".into()),
        Token::Ident("line-through".into()),
    ]);
    assert_eq!(
        parsed_dashed,
        Some(
            TextDecoration::initial()
                .with_line(TextDecorationLine::LINE_THROUGH)
                .with_style(TextDecorationStyle::Dashed)
        )
    );
}

// -----------------------------------------------------------------------------
// 6. text-transform
// -----------------------------------------------------------------------------

#[test]
fn text_transform_behavior_and_parsing() {
    assert_eq!(TextTransform::None.apply("alloy browser"), "alloy browser");
    assert_eq!(
        TextTransform::Uppercase.apply("alloy browser"),
        "ALLOY BROWSER"
    );
    assert_eq!(
        TextTransform::Lowercase.apply("ALLOY BROWSER"),
        "alloy browser"
    );
    assert_eq!(
        TextTransform::Capitalize.apply("alloy browser engine"),
        "Alloy Browser Engine"
    );

    assert_eq!(
        parse_text_transform(&[Token::Ident("none".into())]),
        Some(TextTransform::None)
    );
    assert_eq!(
        parse_text_transform(&[Token::Ident("capitalize".into())]),
        Some(TextTransform::Capitalize)
    );
    assert_eq!(
        parse_text_transform(&[Token::Ident("uppercase".into())]),
        Some(TextTransform::Uppercase)
    );
    assert_eq!(
        parse_text_transform(&[Token::Ident("lowercase".into())]),
        Some(TextTransform::Lowercase)
    );
    assert_eq!(parse_text_transform(&[Token::Ident("bold".into())]), None);
}

// -----------------------------------------------------------------------------
// 7. text-overflow
// -----------------------------------------------------------------------------

#[test]
fn text_overflow_variants_and_parsing() {
    assert!(!TextOverflow::Clip.is_ellipsis());
    assert!(TextOverflow::Ellipsis.is_ellipsis());

    assert_eq!(TextOverflow::Clip.keyword(), "clip");
    assert_eq!(TextOverflow::Ellipsis.keyword(), "ellipsis");

    assert_eq!(
        parse_text_overflow(&[Token::Ident("clip".into())]),
        Some(TextOverflow::Clip)
    );
    assert_eq!(
        parse_text_overflow(&[Token::Ident("ellipsis".into())]),
        Some(TextOverflow::Ellipsis)
    );
    assert_eq!(parse_text_overflow(&[Token::Ident("hidden".into())]), None);
}

// -----------------------------------------------------------------------------
// 8. overflow-wrap and word-wrap
// -----------------------------------------------------------------------------

#[test]
fn overflow_wrap_variants_and_parsing() {
    assert!(!OverflowWrap::Normal.allows_emergency_break());
    assert!(OverflowWrap::BreakWord.allows_emergency_break());
    assert!(OverflowWrap::Anywhere.allows_emergency_break());

    assert_eq!(
        parse_overflow_wrap(&[Token::Ident("normal".into())]),
        Some(OverflowWrap::Normal)
    );
    assert_eq!(
        parse_overflow_wrap(&[Token::Ident("break-word".into())]),
        Some(OverflowWrap::BreakWord)
    );
    assert_eq!(
        parse_overflow_wrap(&[Token::Ident("anywhere".into())]),
        Some(OverflowWrap::Anywhere)
    );
    assert_eq!(parse_overflow_wrap(&[Token::Ident("clip".into())]), None);
}

// -----------------------------------------------------------------------------
// 9. word-break
// -----------------------------------------------------------------------------

#[test]
fn word_break_variants_and_parsing() {
    assert!(!WordBreak::Normal.allows_break_all());
    assert!(WordBreak::BreakAll.allows_break_all());
    assert!(!WordBreak::KeepAll.allows_break_all());

    assert_eq!(
        parse_word_break(&[Token::Ident("normal".into())]),
        Some(WordBreak::Normal)
    );
    assert_eq!(
        parse_word_break(&[Token::Ident("break-all".into())]),
        Some(WordBreak::BreakAll)
    );
    assert_eq!(
        parse_word_break(&[Token::Ident("keep-all".into())]),
        Some(WordBreak::KeepAll)
    );
    assert_eq!(parse_word_break(&[Token::Ident("wrap".into())]), None);
}

// -----------------------------------------------------------------------------
// 10. TextAdvanceStyle aggregate, cascade apply, reset, inherit, and tree inheritance
// -----------------------------------------------------------------------------

#[test]
fn text_advance_style_initial_values() {
    let style = TextAdvanceStyle::initial();
    assert_eq!(style.font_weight(), FontWeight::NORMAL);
    assert_eq!(style.font_style(), FontStyle::Normal);
    assert_eq!(style.line_height(), LineHeight::Normal);
    assert_eq!(style.letter_spacing(), LetterSpacing::Normal);
    assert_eq!(style.word_spacing(), WordSpacing::Normal);
    assert_eq!(style.text_decoration(), TextDecoration::initial());
    assert_eq!(style.text_transform(), TextTransform::None);
    assert_eq!(style.text_overflow(), TextOverflow::Clip);
    assert_eq!(style.overflow_wrap(), OverflowWrap::Normal);
    assert_eq!(style.word_break(), WordBreak::Normal);
}

#[test]
fn text_advance_style_cascade_apply_and_reset() {
    let mut style = TextAdvanceStyle::initial();

    // apply font-weight
    style = apply(style, "font-weight", &[Token::Number(600.0)]).unwrap();
    assert_eq!(style.font_weight(), FontWeight::SEMI_BOLD);

    // apply font-style
    style = apply(style, "font-style", &[Token::Ident("italic".into())]).unwrap();
    assert_eq!(style.font_style(), FontStyle::Italic);

    // apply line-height
    style = apply(style, "line-height", &[Token::Number(1.6)]).unwrap();
    assert_eq!(
        style.line_height(),
        LineHeight::Number(LineHeightFactor::new(1.6))
    );

    // apply letter-spacing
    style = apply(
        style,
        "letter-spacing",
        &[Token::Dimension(1.0, "px".into())],
    )
    .unwrap();
    assert_eq!(
        style.letter_spacing(),
        LetterSpacing::Length(Length::Pixels(1.0))
    );

    // apply word-spacing
    style = apply(style, "word-spacing", &[Token::Dimension(2.0, "px".into())]).unwrap();
    assert_eq!(
        style.word_spacing(),
        WordSpacing::Length(Length::Pixels(2.0))
    );

    // apply text-decoration-line
    style = apply(
        style,
        "text-decoration-line",
        &[Token::Ident("underline".into())],
    )
    .unwrap();
    assert!(style.text_decoration().line().has_underline());

    // apply text-decoration-color
    style = apply(
        style,
        "text-decoration-color",
        &[Token::Ident("red".into())],
    )
    .unwrap();
    assert_eq!(style.text_decoration().color(), CssColor::rgb(255, 0, 0));

    // apply text-decoration-style
    style = apply(
        style,
        "text-decoration-style",
        &[Token::Ident("wavy".into())],
    )
    .unwrap();
    assert_eq!(style.text_decoration().style(), TextDecorationStyle::Wavy);

    // apply text-transform
    style = apply(style, "text-transform", &[Token::Ident("uppercase".into())]).unwrap();
    assert_eq!(style.text_transform(), TextTransform::Uppercase);

    // apply text-overflow
    style = apply(style, "text-overflow", &[Token::Ident("ellipsis".into())]).unwrap();
    assert_eq!(style.text_overflow(), TextOverflow::Ellipsis);

    // apply overflow-wrap (and word-wrap alias)
    style = apply(style, "overflow-wrap", &[Token::Ident("break-word".into())]).unwrap();
    assert_eq!(style.overflow_wrap(), OverflowWrap::BreakWord);

    style = apply(style, "word-wrap", &[Token::Ident("anywhere".into())]).unwrap();
    assert_eq!(style.overflow_wrap(), OverflowWrap::Anywhere);

    // apply word-break
    style = apply(style, "word-break", &[Token::Ident("break-all".into())]).unwrap();
    assert_eq!(style.word_break(), WordBreak::BreakAll);

    // reset font-weight
    style = reset(style, "font-weight").unwrap();
    assert_eq!(style.font_weight(), FontWeight::NORMAL);

    // reset text-overflow
    style = reset(style, "text-overflow").unwrap();
    assert_eq!(style.text_overflow(), TextOverflow::Clip);
}

#[test]
fn text_advance_typographic_inheritance() {
    let mut parent = TextAdvanceStyle::initial();
    parent = parent
        .with_font_weight(FontWeight::BOLD)
        .with_font_style(FontStyle::Italic)
        .with_line_height(LineHeight::Number(LineHeightFactor::new(1.8)))
        .with_letter_spacing(LetterSpacing::Length(Length::Pixels(2.0)))
        .with_word_spacing(WordSpacing::Length(Length::Pixels(3.0)))
        .with_text_decoration(TextDecoration::initial().with_line(TextDecorationLine::UNDERLINE))
        .with_text_transform(TextTransform::Capitalize)
        .with_text_overflow(TextOverflow::Ellipsis)
        .with_overflow_wrap(OverflowWrap::BreakWord)
        .with_word_break(WordBreak::BreakAll);

    // Natural CSS tree inheritance:
    // font-weight, font-style, line-height, letter-spacing, word-spacing,
    // text-transform, overflow-wrap, word-break INHERIT.
    // text-decoration and text-overflow DO NOT INHERIT (reset to initial).
    let child = TextAdvanceStyle::inheriting_from(&parent);

    assert_eq!(child.font_weight(), FontWeight::BOLD);
    assert_eq!(child.font_style(), FontStyle::Italic);
    assert_eq!(
        child.line_height(),
        LineHeight::Number(LineHeightFactor::new(1.8))
    );
    assert_eq!(
        child.letter_spacing(),
        LetterSpacing::Length(Length::Pixels(2.0))
    );
    assert_eq!(
        child.word_spacing(),
        WordSpacing::Length(Length::Pixels(3.0))
    );
    assert_eq!(child.text_transform(), TextTransform::Capitalize);
    assert_eq!(child.overflow_wrap(), OverflowWrap::BreakWord);
    assert_eq!(child.word_break(), WordBreak::BreakAll);

    // Non-inherited reset:
    assert_eq!(child.text_decoration(), TextDecoration::initial());
    assert_eq!(child.text_overflow(), TextOverflow::Clip);

    // Explicit inherit keyword on non-inherited property (e.g. text-overflow: inherit):
    let inherited_overflow = inherit(child, parent, "text-overflow").unwrap();
    assert_eq!(inherited_overflow.text_overflow(), TextOverflow::Ellipsis);

    let inherited_decoration = inherit(child, parent, "text-decoration").unwrap();
    assert!(
        inherited_decoration
            .text_decoration()
            .line()
            .has_underline()
    );
}
