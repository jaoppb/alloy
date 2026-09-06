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
        PORT_SCHEMA_VERSION, 7,
        "version 7 adds visual decorations, advanced typography, grid, and logical properties (ADR-0011 item 3)"
    );
    assert_eq!(SUPPORTED_PROPERTIES.len(), 133);
    assert!(SUPPORTED_PROPERTIES.contains(&"font-size"));
    assert!(SUPPORTED_PROPERTIES.contains(&"font-family"));
    assert!(SUPPORTED_PROPERTIES.contains(&"margin-left"));
    assert!(SUPPORTED_PROPERTIES.contains(&"background"));
    assert!(SUPPORTED_PROPERTIES.contains(&"border"));
    assert!(SUPPORTED_PROPERTIES.contains(&"position"));
    assert!(SUPPORTED_PROPERTIES.contains(&"top"));
    assert!(SUPPORTED_PROPERTIES.contains(&"z-index"));
    assert!(SUPPORTED_PROPERTIES.contains(&"min-width"));
    assert!(SUPPORTED_PROPERTIES.contains(&"overflow"));
    assert!(SUPPORTED_PROPERTIES.contains(&"border-radius"));
    assert!(SUPPORTED_PROPERTIES.contains(&"opacity"));
    assert!(SUPPORTED_PROPERTIES.contains(&"font-weight"));
    assert!(SUPPORTED_PROPERTIES.contains(&"grid-template-columns"));
    assert!(SUPPORTED_PROPERTIES.contains(&"writing-mode"));
    assert_eq!(SUPPORTED_SELECTORS.len(), 19);
    assert!(SUPPORTED_SELECTORS.contains(&":nth-child()"));
}

// ---- Positioning & Insets ----

#[test]
fn position_style_defaults_and_builders() {
    use css::{PositionStyle, PositionType, Sizing, ZIndex};

    let initial = PositionStyle::initial();
    assert_eq!(initial.position(), PositionType::Static);
    assert!(initial.position().is_in_flow());
    assert!(!initial.position().is_positioned());
    assert_eq!(initial.top(), Sizing::Auto);
    assert_eq!(initial.right(), Sizing::Auto);
    assert_eq!(initial.bottom(), Sizing::Auto);
    assert_eq!(initial.left(), Sizing::Auto);
    assert_eq!(initial.z_index(), ZIndex::Auto);

    let custom = initial
        .with_position(PositionType::Absolute)
        .with_top(Sizing::Fixed(Length::pixels(10.0)))
        .with_right(Sizing::Fixed(Length::pixels(20.0)))
        .with_bottom(Sizing::Fixed(Length::pixels(30.0)))
        .with_left(Sizing::Fixed(Length::pixels(40.0)))
        .with_z_index(ZIndex::Index(99));

    assert_eq!(custom.position(), PositionType::Absolute);
    assert!(!custom.position().is_in_flow());
    assert!(custom.position().is_positioned());
    assert_eq!(custom.top(), Sizing::Fixed(Length::pixels(10.0)));
    assert_eq!(custom.right(), Sizing::Fixed(Length::pixels(20.0)));
    assert_eq!(custom.bottom(), Sizing::Fixed(Length::pixels(30.0)));
    assert_eq!(custom.left(), Sizing::Fixed(Length::pixels(40.0)));
    assert_eq!(custom.z_index(), ZIndex::Index(99));
    assert_eq!(custom.position().to_string(), "absolute");
    assert_eq!(custom.z_index().to_string(), "99");
    assert_eq!(ZIndex::Auto.to_string(), "auto");
    assert_eq!(PositionType::Relative.to_string(), "relative");
    assert_eq!(PositionType::Fixed.to_string(), "fixed");
    assert_eq!(PositionType::Sticky.to_string(), "sticky");
}

// ---- Overflow ----

#[test]
fn overflow_style_defaults_and_builders() {
    use css::{Overflow, OverflowStyle};

    let initial = OverflowStyle::initial();
    assert_eq!(initial.x(), Overflow::Visible);
    assert_eq!(initial.y(), Overflow::Visible);
    assert!(!initial.x().is_clipped());

    let custom = initial.with_x(Overflow::Hidden).with_y(Overflow::Auto);
    assert_eq!(custom.x(), Overflow::Hidden);
    assert_eq!(custom.y(), Overflow::Auto);
    assert!(custom.x().is_clipped());
    assert!(custom.y().is_clipped());
    assert_eq!(custom.x().to_string(), "hidden");
    assert_eq!(Overflow::Clip.to_string(), "clip");
    assert_eq!(Overflow::Scroll.to_string(), "scroll");
    assert_eq!(Overflow::Auto.to_string(), "auto");

    let uniform = OverflowStyle::uniform(Overflow::Scroll);
    assert_eq!(uniform.x(), Overflow::Scroll);
    assert_eq!(uniform.y(), Overflow::Scroll);
}

// ---- SizingConstraints ----

#[test]
fn sizing_constraints_defaults_and_builders() {
    use css::{Sizing, SizingConstraints};

    let initial = SizingConstraints::initial();
    assert_eq!(initial.min_width(), Sizing::Auto);
    assert_eq!(initial.max_width(), Sizing::Auto);
    assert_eq!(initial.min_height(), Sizing::Auto);
    assert_eq!(initial.max_height(), Sizing::Auto);

    let custom = initial
        .with_min_width(Sizing::Fixed(Length::pixels(100.0)))
        .with_max_width(Sizing::Fixed(Length::pixels(800.0)))
        .with_min_height(Sizing::Fixed(Length::pixels(50.0)))
        .with_max_height(Sizing::Fixed(Length::pixels(400.0)));

    assert_eq!(custom.min_width(), Sizing::Fixed(Length::pixels(100.0)));
    assert_eq!(custom.max_width(), Sizing::Fixed(Length::pixels(800.0)));
    assert_eq!(custom.min_height(), Sizing::Fixed(Length::pixels(50.0)));
    assert_eq!(custom.max_height(), Sizing::Fixed(Length::pixels(400.0)));
}
