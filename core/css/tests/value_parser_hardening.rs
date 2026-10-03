//! Regression tests for the hardened value parsers: grid track-list bounds
//! (author-controlled `repeat()` counts), case-sensitive grid names, and
//! comma-aware `box-shadow` / functional border colours.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::{Duration, Instant};

use css::domain::color::CssColor;
use css::domain::computed::grid::{GridPlacement, GridStyle, TrackSize};
use css::domain::computed::visual::{BoxShadowList, ShadowPlacement, VisualStyle};
use css::domain::length::Length;
use css::infrastructure::cascade::grid_values::{self, tokenize_value};
use css::infrastructure::cascade::visual_values::{
    apply_visual_property, parse_border_color, parse_border_color_shorthand, parse_box_shadow,
};

/// Generous enough for any CI machine; the unbounded expansion this guards
/// against takes minutes or aborts the process.
const QUICK: Duration = Duration::from_secs(2);

fn track_list(source: &str) -> Option<css::domain::computed::grid::TrackList> {
    grid_values::parse_track_list(&tokenize_value(source))
}

// ---- #1: repeat() bounds -------------------------------------------------

#[test]
fn repeat_with_huge_count_is_rejected_without_allocating() {
    let started = Instant::now();
    assert!(track_list("repeat(4000000000, 1px 1px)").is_none());
    assert!(started.elapsed() < QUICK);
}

#[test]
fn huge_repeat_through_the_cascade_leaves_the_template_untouched() {
    let tokens = tokenize_value("repeat(4000000000, 1fr 1fr)");
    let applied = grid_values::apply(GridStyle::initial(), "grid-template-columns", &tokens);
    assert!(applied.is_none());
}

#[test]
fn nested_repeat_is_rejected() {
    assert!(track_list("repeat(2, repeat(2, 1px))").is_none());
    assert!(track_list("10px repeat(2, 5px repeat(2, 1px))").is_none());
}

#[test]
fn nested_huge_repeat_is_rejected_quickly() {
    let started = Instant::now();
    assert!(track_list("repeat(100000, repeat(100000, 1px))").is_none());
    assert!(started.elapsed() < QUICK);
}

#[test]
fn repeat_past_the_track_capacity_is_rejected_not_truncated() {
    assert!(track_list("repeat(17, 1px)").is_none());
    assert!(track_list("repeat(9, 1px 2px)").is_none());
    assert!(track_list("repeat(8, 1px) repeat(9, 1px)").is_none());
}

#[test]
fn repeat_up_to_the_track_capacity_is_accepted() {
    let tracks = track_list("repeat(16, 1px)").expect("16 tracks fit");
    assert_eq!(tracks.len(), 16);
    assert!(
        tracks
            .tracks()
            .iter()
            .all(|track| *track == TrackSize::pixels(1.0))
    );

    let alternating = track_list("repeat(8, 1px 2px)").expect("16 tracks fit");
    assert_eq!(alternating.len(), 16);
    assert_eq!(alternating.get(14), Some(TrackSize::pixels(1.0)));
    assert_eq!(alternating.get(15), Some(TrackSize::pixels(2.0)));
}

#[test]
fn plain_track_list_past_the_capacity_is_rejected() {
    let sixteen = ["1px"; 16].join(" ");
    assert_eq!(track_list(&sixteen).map(|tracks| tracks.len()), Some(16));
    let seventeen = ["1px"; 17].join(" ");
    assert!(track_list(&seventeen).is_none());
}

#[test]
fn repeat_keeps_a_minmax_argument_whole() {
    let tracks = track_list("repeat(2, minmax(10px, 1fr))").expect("minmax repeats");
    assert_eq!(tracks.len(), 2);
    assert!(tracks.tracks().iter().all(TrackSize::is_flexible));
}

#[test]
fn repeat_with_extra_arguments_is_rejected() {
    assert!(track_list("repeat(2, 1px, 2px)").is_none());
    assert!(track_list("repeat(0, 1px)").is_none());
    assert!(track_list("repeat(1.5, 1px)").is_none());
}

// ---- #8: case-sensitive grid names ---------------------------------------

#[test]
fn template_area_names_are_case_sensitive() {
    let tokens = tokenize_value("\"Nav nav\"");
    let areas = grid_values::parse_grid_template_areas(&tokens).expect("two areas");
    let upper = areas.find_area("Nav").expect("Nav exists");
    let lower = areas.find_area("nav").expect("nav exists");
    assert_ne!(upper, lower);
    assert_eq!((upper.column_start(), upper.column_end()), (1, 2));
    assert_eq!((lower.column_start(), lower.column_end()), (2, 3));
    assert!(areas.find_area("NAV").is_none());
    assert_eq!(format!("{areas}"), "\"Nav nav\"");
}

#[test]
fn line_names_are_case_sensitive() {
    let tokens = tokenize_value("Header");
    let placement = grid_values::parse_grid_line_placement(&tokens).expect("named line");
    assert_eq!(placement, GridPlacement::named("Header").unwrap());
    assert_ne!(placement, GridPlacement::named("header").unwrap());
}

#[test]
fn reserved_grid_keywords_stay_case_insensitive() {
    assert!(GridPlacement::named("AUTO").is_none());
    assert!(GridPlacement::named("Span").is_none());
    let tokens = tokenize_value("\"None a\"");
    assert!(grid_values::parse_grid_template_areas(&tokens).is_none());
}

#[test]
fn over_long_grid_names_are_rejected() {
    let long_name = "a".repeat(33);
    let areas = tokenize_value(&format!("\"{long_name}\""));
    assert!(grid_values::parse_grid_template_areas(&areas).is_none());
    let line = tokenize_value(&long_name);
    assert!(grid_values::parse_grid_line_placement(&line).is_none());
}

#[test]
fn template_areas_past_the_cell_capacity_are_rejected() {
    let row = format!("\"{}\"", ["a"; 13].join(" "));
    let five_rows = [row.as_str(); 5].join(" ");
    assert!(grid_values::parse_grid_template_areas(&tokenize_value(&five_rows)).is_none());
}

// ---- #4: box-shadow -------------------------------------------------------

fn shadows(source: &str) -> Option<BoxShadowList> {
    parse_box_shadow(&tokenize_value(source))
}

#[test]
fn box_shadow_keeps_an_rgba_colour_whole() {
    let list = shadows("0 1px 3px rgba(0,0,0,.2)").expect("one shadow");
    let [Some(shadow), None, None, None] = list.entries() else {
        panic!("expected exactly one shadow, got {list:?}");
    };
    assert_eq!(shadow.horizontal(), Length::ZERO);
    assert_eq!(shadow.vertical(), Length::Pixels(1.0));
    assert_eq!(shadow.blur(), Length::Pixels(3.0));
    assert_eq!(shadow.color(), CssColor::rgba(0, 0, 0, 51));
}

#[test]
fn box_shadow_list_with_functional_colours() {
    let list = shadows("0 1px 2px rgb(1,2,3), inset 0 0 4px red").expect("two shadows");
    let [Some(first), Some(second), None, None] = list.entries() else {
        panic!("expected exactly two shadows, got {list:?}");
    };
    assert_eq!(first.color(), CssColor::rgb(1, 2, 3));
    assert_eq!(first.placement(), ShadowPlacement::Outset);
    assert_eq!(second.color(), CssColor::rgb(0xFF, 0, 0));
    assert_eq!(second.placement(), ShadowPlacement::Inset);
    assert_eq!(second.blur(), Length::Pixels(4.0));
}

#[test]
fn box_shadow_rejects_unknown_and_repeated_components() {
    assert!(shadows("2px 2px bogus").is_none());
    assert!(shadows("2px 2px 2px 2px 2px").is_none());
    assert!(shadows("2px 2px red blue").is_none());
    assert!(shadows("inset 2px 2px inset").is_none());
    assert!(shadows("2px").is_none());
    assert!(shadows("2px 2px calc(1px)").is_none());
    assert!(shadows("").is_none());
    assert!(shadows("2px 2px,").is_none());
}

#[test]
fn box_shadow_past_the_capacity_is_rejected_not_truncated() {
    let four = ["1px 1px"; 4].join(", ");
    assert!(shadows(&four).is_some());
    let five = ["1px 1px"; 5].join(", ");
    assert!(shadows(&five).is_none());
    let style = apply_visual_property(VisualStyle::initial(), "box-shadow", &tokenize_value(&five));
    assert!(style.is_none());
}

// ---- #10: border colours through the canonical colour parser --------------

#[test]
fn border_color_accepts_functional_colours() {
    let translucent = CssColor::rgba(0, 0, 0, 128);
    let edges = parse_border_color_shorthand(&tokenize_value("rgba(0,0,0,.5)")).expect("rgba");
    assert_eq!(edges.top(), translucent);
    assert_eq!(edges.left(), translucent);

    let pair =
        parse_border_color_shorthand(&tokenize_value("red rgb(1, 2, 3)")).expect("two colours");
    assert_eq!(pair.top(), CssColor::rgb(0xFF, 0, 0));
    assert_eq!(pair.right(), CssColor::rgb(1, 2, 3));
    assert_eq!(pair.bottom(), CssColor::rgb(0xFF, 0, 0));

    let top = parse_border_color(&tokenize_value("rgb(1,2,3)"));
    assert_eq!(top, Some(CssColor::rgb(1, 2, 3)));

    let style = apply_visual_property(
        VisualStyle::initial(),
        "border-top-color",
        &tokenize_value("rgb(1,2,3)"),
    )
    .expect("border-top-color applies");
    assert_eq!(style.border_colors().top(), CssColor::rgb(1, 2, 3));
}
