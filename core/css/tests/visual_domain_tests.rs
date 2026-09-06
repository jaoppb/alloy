//! Comprehensive tests for visual decoration domain types and parsers (Track 2).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

#[path = "../src/domain/computed/visual.rs"]
pub mod visual;

pub mod domain {
    pub use css::domain::*;
    pub mod computed {
        pub use crate::visual;
        pub use css::domain::computed::*;
    }
}

pub use css::infrastructure;

#[path = "../src/infrastructure/cascade/visual_values.rs"]
pub mod visual_values;

use css::domain::color::CssColor;
use css::domain::computed::sizing::Sizing;
use css::domain::length::Length;
use css::infrastructure::parser::tokenize;

use visual::{
    BackgroundImage, BackgroundPosition, BackgroundRepeat, BackgroundSize, BorderColorEdges,
    BorderRadius, BorderStyle, BorderStyleEdges, BoxShadow, BoxShadowList, Opacity,
    ShadowPlacement, VisualStyle,
};
use visual_values::{
    apply_visual_property, inherit_visual_property, parse_background_image,
    parse_background_position, parse_background_repeat, parse_background_size, parse_border_color,
    parse_border_color_shorthand, parse_border_radius_corner, parse_border_radius_shorthand,
    parse_border_style, parse_border_style_shorthand, parse_box_shadow, parse_opacity,
    reset_visual_property,
};

fn tokens_of(source: &str) -> Vec<css::infrastructure::parser::Token> {
    tokenize(source)
        .iter()
        .map(|spanned| spanned.token().clone())
        .filter(|token| !token.is_whitespace())
        .collect()
}

// =========================================================================
// 1. Border Style Tests
// =========================================================================

#[test]
fn test_border_style_initial_and_keywords() {
    assert_eq!(BorderStyle::default(), BorderStyle::None);
    assert_eq!(BorderStyle::None.keyword(), "none");
    assert!(BorderStyle::None.is_none_or_hidden());
    assert!(BorderStyle::Hidden.is_none_or_hidden());
    assert!(!BorderStyle::Solid.is_none_or_hidden());

    let all_keywords = [
        ("none", BorderStyle::None),
        ("hidden", BorderStyle::Hidden),
        ("dotted", BorderStyle::Dotted),
        ("dashed", BorderStyle::Dashed),
        ("solid", BorderStyle::Solid),
        ("double", BorderStyle::Double),
        ("groove", BorderStyle::Groove),
        ("ridge", BorderStyle::Ridge),
        ("inset", BorderStyle::Inset),
        ("outset", BorderStyle::Outset),
    ];
    for (kw, expected) in all_keywords {
        assert_eq!(BorderStyle::from_keyword(kw), Some(expected));
        assert_eq!(expected.keyword(), kw);
        assert_eq!(format!("{expected}"), kw);
    }
    assert_eq!(BorderStyle::from_keyword("invalid"), None);
    assert_eq!(
        parse_border_style(&tokens_of("solid")),
        Some(BorderStyle::Solid)
    );
    assert_eq!(
        parse_border_style(&tokens_of("dashed")),
        Some(BorderStyle::Dashed)
    );
}

#[test]
fn test_border_style_edges_immutability() {
    let edges = BorderStyleEdges::NONE;
    assert_eq!(edges.top(), BorderStyle::None);
    assert_eq!(edges.right(), BorderStyle::None);
    assert_eq!(edges.bottom(), BorderStyle::None);
    assert_eq!(edges.left(), BorderStyle::None);

    let updated = edges
        .with_top(BorderStyle::Solid)
        .with_right(BorderStyle::Dashed)
        .with_bottom(BorderStyle::Dotted)
        .with_left(BorderStyle::Double);

    // Original remains unchanged
    assert_eq!(edges.top(), BorderStyle::None);
    // Updated has new values
    assert_eq!(updated.top(), BorderStyle::Solid);
    assert_eq!(updated.right(), BorderStyle::Dashed);
    assert_eq!(updated.bottom(), BorderStyle::Dotted);
    assert_eq!(updated.left(), BorderStyle::Double);

    let uniform = BorderStyleEdges::uniform(BorderStyle::Groove);
    assert_eq!(uniform.top(), BorderStyle::Groove);
    assert_eq!(uniform.right(), BorderStyle::Groove);
    assert_eq!(uniform.bottom(), BorderStyle::Groove);
    assert_eq!(uniform.left(), BorderStyle::Groove);
}

#[test]
fn test_parse_border_style_shorthands() {
    let t1 = tokens_of("solid");
    let res1 = parse_border_style_shorthand(&t1).unwrap();
    assert_eq!(res1, BorderStyleEdges::uniform(BorderStyle::Solid));

    let t2 = tokens_of("solid dashed");
    let res2 = parse_border_style_shorthand(&t2).unwrap();
    assert_eq!(
        res2,
        BorderStyleEdges::new(
            BorderStyle::Solid,
            BorderStyle::Dashed,
            BorderStyle::Solid,
            BorderStyle::Dashed
        )
    );

    let t3 = tokens_of("solid dashed dotted");
    let res3 = parse_border_style_shorthand(&t3).unwrap();
    assert_eq!(
        res3,
        BorderStyleEdges::new(
            BorderStyle::Solid,
            BorderStyle::Dashed,
            BorderStyle::Dotted,
            BorderStyle::Dashed
        )
    );

    let t4 = tokens_of("solid dashed dotted double");
    let res4 = parse_border_style_shorthand(&t4).unwrap();
    assert_eq!(
        res4,
        BorderStyleEdges::new(
            BorderStyle::Solid,
            BorderStyle::Dashed,
            BorderStyle::Dotted,
            BorderStyle::Double
        )
    );

    let t_inv = tokens_of("solid dashed dotted double outset");
    assert_eq!(parse_border_style_shorthand(&t_inv), None);
}

// =========================================================================
// 2. Border Color Tests
// =========================================================================

#[test]
fn test_border_color_initial_and_immutability() {
    let initial = BorderColorEdges::default();
    assert_eq!(initial.top(), CssColor::BLACK);
    assert_eq!(initial.right(), CssColor::BLACK);
    assert_eq!(initial.bottom(), CssColor::BLACK);
    assert_eq!(initial.left(), CssColor::BLACK);

    let red = CssColor::rgb(0xFF, 0x00, 0x00);
    let blue = CssColor::rgb(0x00, 0x00, 0xFF);
    let green = CssColor::rgb(0x00, 0x80, 0x00);
    let white = CssColor::rgb(0xFF, 0xFF, 0xFF);

    let modified = initial
        .with_top(red)
        .with_right(blue)
        .with_bottom(green)
        .with_left(white);

    assert_eq!(initial.top(), CssColor::BLACK);
    assert_eq!(modified.top(), red);
    assert_eq!(modified.right(), blue);
    assert_eq!(modified.bottom(), green);
    assert_eq!(modified.left(), white);
}

#[test]
fn test_parse_border_color_formats() {
    let red = CssColor::rgb(0xFF, 0x00, 0x00);
    let blue = CssColor::rgb(0x00, 0x00, 0xFF);

    assert_eq!(parse_border_color(&tokens_of("red")), Some(red));
    assert_eq!(parse_border_color(&tokens_of("#f00")), Some(red));
    assert_eq!(parse_border_color(&tokens_of("#ff0000")), Some(red));
    assert_eq!(parse_border_color(&tokens_of("blue")), Some(blue));
    assert_eq!(
        parse_border_color(&tokens_of("transparent")),
        Some(CssColor::TRANSPARENT)
    );
    assert_eq!(parse_border_color(&tokens_of("unknowncolor")), None);

    let s1 = parse_border_color_shorthand(&tokens_of("red")).unwrap();
    assert_eq!(s1, BorderColorEdges::uniform(red));

    let s2 = parse_border_color_shorthand(&tokens_of("red blue")).unwrap();
    assert_eq!(s2, BorderColorEdges::new(red, blue, red, blue));
}

// =========================================================================
// 3. Border Radius Tests
// =========================================================================

#[test]
fn test_border_radius_initial_and_immutability() {
    let radius = BorderRadius::initial();
    assert!(radius.is_zero());
    assert_eq!(radius.top_left(), Length::ZERO);
    assert_eq!(radius.top_right(), Length::ZERO);
    assert_eq!(radius.bottom_right(), Length::ZERO);
    assert_eq!(radius.bottom_left(), Length::ZERO);

    let l10 = Length::Pixels(10.0);
    let l20 = Length::Pixels(20.0);
    let updated = radius.with_top_left(l10).with_bottom_right(l20);

    assert!(radius.is_zero());
    assert!(!updated.is_zero());
    assert_eq!(updated.top_left(), l10);
    assert_eq!(updated.top_right(), Length::ZERO);
    assert_eq!(updated.bottom_right(), l20);
    assert_eq!(updated.bottom_left(), Length::ZERO);
}

#[test]
fn test_parse_border_radius_shorthands() {
    let l10 = Length::Pixels(10.0);
    let l20 = Length::Pixels(20.0);
    let l30 = Length::Pixels(30.0);
    let l40 = Length::Pixels(40.0);

    assert_eq!(parse_border_radius_corner(&tokens_of("10px")), Some(l10));
    assert_eq!(
        parse_border_radius_corner(&tokens_of("0")),
        Some(Length::ZERO)
    );

    let r1 = parse_border_radius_shorthand(&tokens_of("10px")).unwrap();
    assert_eq!(r1, BorderRadius::uniform(l10));

    let r2 = parse_border_radius_shorthand(&tokens_of("10px 20px")).unwrap();
    assert_eq!(r2, BorderRadius::new(l10, l20, l10, l20));

    let r3 = parse_border_radius_shorthand(&tokens_of("10px 20px 30px")).unwrap();
    assert_eq!(r3, BorderRadius::new(l10, l20, l30, l20));

    let r4 = parse_border_radius_shorthand(&tokens_of("10px 20px 30px 40px")).unwrap();
    assert_eq!(r4, BorderRadius::new(l10, l20, l30, l40));
}

// =========================================================================
// 4. Box Shadow Tests
// =========================================================================

#[test]
fn test_box_shadow_initial_and_parsing() {
    let list = BoxShadowList::none();
    assert!(list.is_none());

    let direct_shadow = BoxShadow::new(
        Length::Pixels(1.0),
        Length::Pixels(2.0),
        Length::ZERO,
        Length::ZERO,
        CssColor::BLACK,
        ShadowPlacement::Outset,
    );
    let single_list = BoxShadowList::from_single(direct_shadow);
    assert!(!single_list.is_none());

    let parsed_none = parse_box_shadow(&tokens_of("none")).unwrap();
    assert!(parsed_none.is_none());

    let shadow2 = parse_box_shadow(&tokens_of("10px 20px")).unwrap();
    assert!(!shadow2.is_none());
    let entry0 = shadow2.entries()[0].unwrap();
    assert_eq!(entry0.horizontal(), Length::Pixels(10.0));
    assert_eq!(entry0.vertical(), Length::Pixels(20.0));
    assert_eq!(entry0.blur(), Length::ZERO);
    assert_eq!(entry0.spread(), Length::ZERO);
    assert_eq!(entry0.color(), CssColor::BLACK);
    assert_eq!(entry0.placement(), ShadowPlacement::Outset);

    let shadow_full = parse_box_shadow(&tokens_of("inset 5px 10px 15px 20px red")).unwrap();
    let full = shadow_full.entries()[0].unwrap();
    assert_eq!(full.horizontal(), Length::Pixels(5.0));
    assert_eq!(full.vertical(), Length::Pixels(10.0));
    assert_eq!(full.blur(), Length::Pixels(15.0));
    assert_eq!(full.spread(), Length::Pixels(20.0));
    assert_eq!(full.color(), CssColor::rgb(0xFF, 0x00, 0x00));
    assert_eq!(full.placement(), ShadowPlacement::Inset);
    assert!(full.placement().is_inset());

    let multi = parse_box_shadow(&tokens_of("2px 2px black, inset 4px 4px blue")).unwrap();
    assert!(multi.entries()[0].is_some());
    assert!(multi.entries()[1].is_some());
    assert!(multi.entries()[2].is_none());
}

// =========================================================================
// 5. Opacity Tests
// =========================================================================

#[test]
fn test_opacity_values_and_clamping() {
    let initial = Opacity::default();
    assert_eq!(initial, Opacity::ONE);
    assert!(initial.is_opaque());
    assert!(!initial.is_transparent());
    assert_eq!(initial.value(), 1.0);

    let zero = Opacity::ZERO;
    assert!(zero.is_transparent());
    assert_eq!(zero.value(), 0.0);

    assert_eq!(Opacity::clamped(1.5).value(), 1.0);
    assert_eq!(Opacity::clamped(-0.2).value(), 0.0);
    assert_eq!(Opacity::clamped(0.75).value(), 0.75);
    assert_eq!(Opacity::clamped(f32::NAN), Opacity::ONE);

    assert_eq!(Opacity::new(f32::NAN), None);
    assert_eq!(Opacity::new(f32::INFINITY), None);

    assert_eq!(parse_opacity(&tokens_of("1")), Some(Opacity::ONE));
    assert_eq!(parse_opacity(&tokens_of("0")), Some(Opacity::ZERO));
    assert_eq!(parse_opacity(&tokens_of("0.5")).unwrap().value(), 0.5);
    assert_eq!(parse_opacity(&tokens_of("80%")).unwrap().value(), 0.8);
    assert_eq!(parse_opacity(&tokens_of("150%")), Some(Opacity::ONE));
}

// =========================================================================
// 6. Background Properties Tests
// =========================================================================

#[test]
fn test_background_image_and_source() {
    let initial = BackgroundImage::default();
    assert!(initial.is_none());
    assert_eq!(initial.url(), None);

    let parsed_none = parse_background_image(&tokens_of("none")).unwrap();
    assert!(parsed_none.is_none());

    let parsed_url1 = parse_background_image(&tokens_of("url(\"hero.png\")")).unwrap();
    assert!(!parsed_url1.is_none());
    assert_eq!(parsed_url1.url().unwrap().as_str(), "hero.png");

    let parsed_url2 = parse_background_image(&tokens_of("url(test.jpg)")).unwrap();
    assert_eq!(parsed_url2.url().unwrap().as_str(), "test.jpg");
}

#[test]
fn test_background_position_and_keywords() {
    let initial = BackgroundPosition::default();
    assert_eq!(initial, BackgroundPosition::INITIAL);
    assert_eq!(initial.x(), Length::Percent(0.0));
    assert_eq!(initial.y(), Length::Percent(0.0));

    let center = parse_background_position(&tokens_of("center")).unwrap();
    assert_eq!(center.x(), Length::Percent(50.0));
    assert_eq!(center.y(), Length::Percent(50.0));

    let right_top = parse_background_position(&tokens_of("right top")).unwrap();
    assert_eq!(right_top.x(), Length::Percent(100.0));
    assert_eq!(right_top.y(), Length::Percent(0.0));

    let lengths = parse_background_position(&tokens_of("10px 20px")).unwrap();
    assert_eq!(lengths.x(), Length::Pixels(10.0));
    assert_eq!(lengths.y(), Length::Pixels(20.0));
}

#[test]
fn test_background_size_and_keywords() {
    let initial = BackgroundSize::default();
    assert!(initial.is_auto());

    assert!(
        parse_background_size(&tokens_of("cover"))
            .unwrap()
            .is_cover()
    );
    assert!(
        parse_background_size(&tokens_of("contain"))
            .unwrap()
            .is_contain()
    );
    assert!(parse_background_size(&tokens_of("auto")).unwrap().is_auto());

    let explicit2 = parse_background_size(&tokens_of("100px 50px")).unwrap();
    assert_eq!(
        explicit2,
        BackgroundSize::explicit(
            Sizing::Fixed(Length::Pixels(100.0)),
            Sizing::Fixed(Length::Pixels(50.0))
        )
    );

    let explicit1 = parse_background_size(&tokens_of("100px")).unwrap();
    assert_eq!(
        explicit1,
        BackgroundSize::explicit(Sizing::Fixed(Length::Pixels(100.0)), Sizing::Auto)
    );
}

#[test]
fn test_background_repeat_and_keywords() {
    let initial = BackgroundRepeat::default();
    assert_eq!(initial, BackgroundRepeat::Repeat);
    assert!(initial.repeats_x());
    assert!(initial.repeats_y());

    let rx = parse_background_repeat(&tokens_of("repeat-x")).unwrap();
    assert_eq!(rx, BackgroundRepeat::RepeatX);
    assert!(rx.repeats_x());
    assert!(!rx.repeats_y());

    let ry = parse_background_repeat(&tokens_of("repeat-y")).unwrap();
    assert_eq!(ry, BackgroundRepeat::RepeatY);
    assert!(!ry.repeats_x());
    assert!(ry.repeats_y());

    let no_rep = parse_background_repeat(&tokens_of("no-repeat")).unwrap();
    assert_eq!(no_rep, BackgroundRepeat::NoRepeat);
    assert!(!no_rep.repeats_x());
    assert!(!no_rep.repeats_y());

    let two_words = parse_background_repeat(&tokens_of("repeat no-repeat")).unwrap();
    assert_eq!(two_words, BackgroundRepeat::RepeatX);
}

// =========================================================================
// 7. VisualStyle Aggregate & Cascade Dispatch Tests
// =========================================================================

#[test]
fn test_visual_style_aggregate_initial() {
    let style = VisualStyle::initial();
    assert_eq!(style.border_styles(), BorderStyleEdges::NONE);
    assert_eq!(style.border_colors(), BorderColorEdges::BLACK);
    assert_eq!(style.border_radius(), BorderRadius::ZERO);
    assert!(style.box_shadow().is_none());
    assert_eq!(style.opacity(), Opacity::ONE);
    assert!(style.background_image().is_none());
    assert_eq!(style.background_position(), BackgroundPosition::INITIAL);
    assert_eq!(style.background_size(), BackgroundSize::Auto);
    assert_eq!(style.background_repeat(), BackgroundRepeat::Repeat);
}

#[test]
fn test_visual_style_builders_immutability() {
    let initial = VisualStyle::initial();
    let red = CssColor::rgb(0xFF, 0x00, 0x00);

    let modified = initial
        .with_border_styles(BorderStyleEdges::uniform(BorderStyle::Solid))
        .with_border_colors(BorderColorEdges::uniform(red))
        .with_border_radius(BorderRadius::uniform(Length::Pixels(8.0)))
        .with_opacity(Opacity::clamped(0.5))
        .with_background_repeat(BackgroundRepeat::NoRepeat);

    // Initial is completely untouched
    assert_eq!(initial.border_styles(), BorderStyleEdges::NONE);
    assert_eq!(initial.opacity(), Opacity::ONE);

    // Modified has all changes
    assert_eq!(
        modified.border_styles(),
        BorderStyleEdges::uniform(BorderStyle::Solid)
    );
    assert_eq!(modified.border_colors(), BorderColorEdges::uniform(red));
    assert_eq!(
        modified.border_radius(),
        BorderRadius::uniform(Length::Pixels(8.0))
    );
    assert_eq!(modified.opacity().value(), 0.5);
    assert_eq!(modified.background_repeat(), BackgroundRepeat::NoRepeat);
}

#[test]
fn test_apply_all_21_visual_properties() {
    let mut style = VisualStyle::initial();

    // 1-5: border styles
    style = apply_visual_property(style, "border-style", &tokens_of("solid")).unwrap();
    assert_eq!(
        style.border_styles(),
        BorderStyleEdges::uniform(BorderStyle::Solid)
    );

    style = apply_visual_property(style, "border-top-style", &tokens_of("dashed")).unwrap();
    assert_eq!(style.border_styles().top(), BorderStyle::Dashed);

    style = apply_visual_property(style, "border-right-style", &tokens_of("dotted")).unwrap();
    assert_eq!(style.border_styles().right(), BorderStyle::Dotted);

    style = apply_visual_property(style, "border-bottom-style", &tokens_of("double")).unwrap();
    assert_eq!(style.border_styles().bottom(), BorderStyle::Double);

    style = apply_visual_property(style, "border-left-style", &tokens_of("groove")).unwrap();
    assert_eq!(style.border_styles().left(), BorderStyle::Groove);

    // 6-10: border colors
    let red = CssColor::rgb(0xFF, 0x00, 0x00);
    let blue = CssColor::rgb(0x00, 0x00, 0xFF);
    let green = CssColor::rgb(0x00, 0x80, 0x00);
    let white = CssColor::rgb(0xFF, 0xFF, 0xFF);

    style = apply_visual_property(style, "border-color", &tokens_of("red")).unwrap();
    assert_eq!(style.border_colors(), BorderColorEdges::uniform(red));

    style = apply_visual_property(style, "border-top-color", &tokens_of("blue")).unwrap();
    assert_eq!(style.border_colors().top(), blue);

    style = apply_visual_property(style, "border-right-color", &tokens_of("green")).unwrap();
    assert_eq!(style.border_colors().right(), green);

    style = apply_visual_property(style, "border-bottom-color", &tokens_of("white")).unwrap();
    assert_eq!(style.border_colors().bottom(), white);

    style = apply_visual_property(style, "border-left-color", &tokens_of("black")).unwrap();
    assert_eq!(style.border_colors().left(), CssColor::BLACK);

    // 11-15: border radius
    style = apply_visual_property(style, "border-radius", &tokens_of("10px")).unwrap();
    assert_eq!(
        style.border_radius(),
        BorderRadius::uniform(Length::Pixels(10.0))
    );

    style = apply_visual_property(style, "border-top-left-radius", &tokens_of("1px")).unwrap();
    assert_eq!(style.border_radius().top_left(), Length::Pixels(1.0));

    style = apply_visual_property(style, "border-top-right-radius", &tokens_of("2px")).unwrap();
    assert_eq!(style.border_radius().top_right(), Length::Pixels(2.0));

    style = apply_visual_property(style, "border-bottom-right-radius", &tokens_of("3px")).unwrap();
    assert_eq!(style.border_radius().bottom_right(), Length::Pixels(3.0));

    style = apply_visual_property(style, "border-bottom-left-radius", &tokens_of("4px")).unwrap();
    assert_eq!(style.border_radius().bottom_left(), Length::Pixels(4.0));

    // 16: box-shadow
    style = apply_visual_property(style, "box-shadow", &tokens_of("2px 4px 6px red")).unwrap();
    assert!(!style.box_shadow().is_none());

    // 17: opacity
    style = apply_visual_property(style, "opacity", &tokens_of("0.8")).unwrap();
    assert_eq!(style.opacity().value(), 0.8);

    // 18: background-image
    style =
        apply_visual_property(style, "background-image", &tokens_of("url(\"bg.png\")")).unwrap();
    assert_eq!(style.background_image().url().unwrap().as_str(), "bg.png");

    // 19: background-position
    style = apply_visual_property(style, "background-position", &tokens_of("50% 50%")).unwrap();
    assert_eq!(style.background_position().x(), Length::Percent(50.0));

    // 20: background-size
    style = apply_visual_property(style, "background-size", &tokens_of("cover")).unwrap();
    assert!(style.background_size().is_cover());

    // 21: background-repeat
    style = apply_visual_property(style, "background-repeat", &tokens_of("no-repeat")).unwrap();
    assert_eq!(style.background_repeat(), BackgroundRepeat::NoRepeat);

    // Reset property
    let reset_op = reset_visual_property(style, "opacity").unwrap();
    assert_eq!(reset_op.opacity(), Opacity::ONE);

    // Inherit property
    let parent = VisualStyle::initial().with_opacity(Opacity::clamped(0.25));
    let inherited = inherit_visual_property(style, &parent, "opacity").unwrap();
    assert_eq!(inherited.opacity().value(), 0.25);
}
