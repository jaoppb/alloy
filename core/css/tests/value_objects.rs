//! Unit coverage for the `domain/` value objects, computed-value enums and the
//! typed error — asserted directly on the types, because that is where the
//! rules (`ADR-0016` fixed-point resolution, `ADR-0011` item 4 typed error with
//! location metadata) are enforced.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use css::{
    Combinator, ComplexSelector, CompoundSelector, ComputedStyle, CssColor, CssError, CssStage,
    DeclarationBlock, Display, EdgeSizes, Identifier, Length, LengthEdges, Origin,
    PORT_SCHEMA_VERSION, SUPPORTED_PROPERTIES, SUPPORTED_SELECTORS, SelectorList, SelectorStep,
    SourceSpan, StyleRule, StyleSheetSet, TypeSelector, ViewportConstraints,
};
use css::{ComputedText, TextMetrics, TextRun};
use graphics::{Au, Color};

const fn au(pixels: i32) -> Au {
    Au::from_whole_px(pixels).unwrap()
}

// ---- Length: the single author-input -> Au crossing ----

#[test]
fn a_pixel_length_resolves_independently_of_context() {
    assert_eq!(
        Length::Pixels(16.0).resolve_to_au(au(99), au(999)),
        Some(au(16))
    );
    assert_eq!(Length::pixels(0.0).magnitude(), 0.0);
    assert_eq!(Length::ZERO, Length::Pixels(0.0));
}

#[test]
fn em_and_rem_scale_the_font_size() {
    assert_eq!(Length::Em(2.0).resolve_to_au(au(16), au(0)), Some(au(32)));
    assert_eq!(Length::Rem(0.5).resolve_to_au(au(16), au(0)), Some(au(8)));
}

#[test]
fn percent_scales_the_container_and_points_convert_at_ninety_six_seventy_seconds() {
    assert_eq!(
        Length::Percent(50.0).resolve_to_au(au(0), au(200)),
        Some(au(100))
    );
    assert_eq!(
        Length::Points(72.0).resolve_to_au(au(0), au(0)),
        Some(au(96))
    );
}

#[test]
fn a_non_finite_length_is_refused_not_substituted() {
    for hostile in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            Length::Pixels(hostile).resolve_to_au(au(16), au(16)),
            None,
            "{hostile} has no correct reading"
        );
    }
}

#[test]
fn length_prints_with_its_unit() {
    assert_eq!(Length::Em(1.5).to_string(), "1.5em");
    assert_eq!(Length::Percent(100.0).to_string(), "100%");
}

// ---- CssColor ----

#[test]
fn css_colour_wraps_a_graphics_colour_both_ways() {
    let colour = CssColor::rgba(0x11, 0x22, 0x33, 0x44);
    assert_eq!(colour.to_graphics(), Color::rgba(0x11, 0x22, 0x33, 0x44));
    assert_eq!(CssColor::from_graphics(Color::BLACK), CssColor::BLACK);
    assert_eq!(CssColor::rgb(1, 2, 3), CssColor::rgba(1, 2, 3, u8::MAX));
    assert_ne!(CssColor::BLACK, CssColor::TRANSPARENT);
}

// ---- Computed values ----

#[test]
fn display_none_suppresses_the_box_and_keywords_round_trip() {
    assert!(Display::None.is_none());
    assert!(!Display::Block.is_none());
    assert_eq!(Display::Flex.keyword(), "flex");
    assert_eq!(Display::default(), Display::Block);
}

#[test]
fn the_initial_computed_style_is_black_transparent_and_sixteen_pixels() {
    let initial = ComputedStyle::initial();
    assert_eq!(initial.color(), CssColor::BLACK);
    assert_eq!(initial.background_color(), CssColor::TRANSPARENT);
    assert_eq!(initial.display(), Display::Block);
    assert_eq!(initial.font_size_au(au(16)), Some(au(16)));
}

#[test]
fn inheriting_carries_colour_and_font_size_but_resets_the_rest() {
    let parent = ComputedStyle::initial()
        .with_color(CssColor::rgb(9, 9, 9))
        .with_font_size(Length::Pixels(20.0))
        .with_margin(LengthEdges::uniform(Length::Pixels(5.0)));
    let child = ComputedStyle::inheriting_from(&parent);

    assert_eq!(child.color(), CssColor::rgb(9, 9, 9), "colour inherits");
    assert_eq!(
        child.font_size_au(au(16)),
        Some(au(20)),
        "font-size inherits"
    );
    assert_eq!(child.margin(), LengthEdges::ZERO, "margin does not inherit");
}

#[test]
fn edge_helpers_build_the_four_sides() {
    let uniform = LengthEdges::uniform(Length::Pixels(4.0));
    assert_eq!(uniform.top(), Length::Pixels(4.0));
    assert_eq!(uniform.left(), Length::Pixels(4.0));

    let vertical = LengthEdges::vertical(Length::Pixels(8.0));
    assert_eq!(vertical.right(), Length::ZERO);
    assert_eq!(vertical.bottom(), Length::Pixels(8.0));
}

#[test]
fn resolved_edge_sizes_sum_across_an_axis() {
    let edges = EdgeSizes::new(au(1), au(2), au(3), au(4));
    assert_eq!(edges.horizontal(), au(6), "left + right");
    assert_eq!(edges.vertical(), au(4), "top + bottom");
    assert_eq!(EdgeSizes::ZERO.horizontal(), Au::ZERO);
}

// ---- Viewport, text vocabulary ----

#[test]
fn viewport_constraints_carry_two_au_lengths() {
    let viewport = ViewportConstraints::new(au(800), au(600));
    assert_eq!(viewport.width(), au(800));
    assert_eq!(viewport.height(), au(600));
}

#[test]
fn text_vocabulary_round_trips() {
    let run = TextRun::new("hello");
    assert_eq!(run.char_count(), 5);
    assert!(!run.is_empty());
    assert_eq!(run.as_str(), "hello");

    let style = ComputedText::new(au(12));
    assert_eq!(style.font_size(), au(12));

    let metrics = TextMetrics::new(au(30), au(14));
    assert_eq!(metrics.width(), au(30));
    assert_eq!(metrics.height(), au(14));
}

// ---- Stylesheet scaffold ----

#[test]
fn a_stylesheet_set_orders_rules_by_origin_and_stays_a_first_class_collection() {
    assert_eq!(Origin::UserAgent.precedence(), 0);
    assert!(Origin::Author.precedence() > Origin::User.precedence());

    let mut block = DeclarationBlock::new();
    block.declare("color", "red");
    let mut sheets = StyleSheetSet::new();
    assert!(sheets.is_empty());
    sheets.push_rule(
        Origin::Author,
        StyleRule::new(type_selector_list("p"), block),
    );

    assert_eq!(sheets.len(), 1);
    let (origin, rule) = sheets.rules().next().expect("one rule");
    assert_eq!(origin, Origin::Author);
    assert_eq!(rule.selectors().to_string(), "p");
    assert_eq!(rule.declarations().len(), 1);
    assert!(
        rule.media().is_always(),
        "a rule outside @media is unconditional"
    );
}

/// The one-element selector list `tag`, built through the domain constructors
/// rather than the parser — this file guards the value objects themselves.
fn type_selector_list(tag: &str) -> SelectorList {
    let mut compound = CompoundSelector::universal();
    let name = Identifier::lowercased(tag).expect("a tag is a valid identifier");
    compound.set_type_selector(TypeSelector::Named(name));
    let step = SelectorStep::new(Combinator::Descendant, compound);
    SelectorList::from_iter([ComplexSelector::new([step])])
}

// ---- Typed error with location metadata (ADR-0011 items 3 & 4) ----

fn any_snapshot_id() -> css::SnapshotId {
    let tree = dom::DomTree::new();
    css::snapshot(&tree, tree.document()).root()
}

const fn assert_is_std_error<E: std::error::Error>(_: &E) {}

#[test]
fn css_error_carries_a_stage_and_an_optional_span() {
    let node = any_snapshot_id();

    let bare = CssError::unknown_node(CssStage::Cascade, node);
    assert_eq!(bare.stage(), CssStage::Cascade);
    assert_eq!(bare.span(), None);

    let located = CssError::unsupported(CssStage::Layout, "no margin collapse yet")
        .with_span(SourceSpan::new(4, 12));
    assert_eq!(located.stage(), CssStage::Layout);
    assert_eq!(located.span(), Some(SourceSpan::new(4, 12)));
    assert_eq!(located.to_string(), "layout stage: no margin collapse yet");
    assert_eq!(
        CssError::missing_computed_style(CssStage::Layout, node).stage(),
        CssStage::Layout
    );
    assert_is_std_error(&bare);
}

#[test]
fn source_span_and_stage_print_readably() {
    assert_eq!(SourceSpan::new(7, 3).to_string(), "7:3");
    assert_eq!(SourceSpan::new(7, 3).line(), 7);
    assert_eq!(SourceSpan::new(7, 3).column(), 3);
    assert_eq!(CssStage::Measure.to_string(), "measure");
}

// ---- Crate-level registries ----

#[test]
fn the_port_schema_version_and_support_registries_are_pinned() {
    assert_eq!(
        PORT_SCHEMA_VERSION, 3,
        "B4 reshaped ComputedStyle, StyledNode and LayoutBox (ADR-0011 item 3)"
    );
    assert_eq!(SUPPORTED_PROPERTIES.len(), 33);
    assert!(SUPPORTED_PROPERTIES.contains(&"font-size"));
    assert!(SUPPORTED_PROPERTIES.contains(&"margin-left"));
    assert_eq!(SUPPORTED_SELECTORS.len(), 19);
    assert!(SUPPORTED_SELECTORS.contains(&":nth-child()"));
}
