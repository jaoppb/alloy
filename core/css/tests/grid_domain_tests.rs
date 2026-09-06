//! Integration and domain tests for CSS Grid Layout Level 1/2.
//! Covers track sizing, template areas, auto-flow, line placement, and gap.

pub mod domain {
    pub use css::domain::*;
    pub mod computed {
        pub use css::domain::computed::*;
        pub mod grid {
            pub use crate::grid::*;
        }
    }
}
pub mod infrastructure {
    pub use css::infrastructure::*;
}

#[path = "../src/domain/computed/grid.rs"]
pub mod grid;

#[path = "../src/infrastructure/cascade/grid_values.rs"]
pub mod grid_values;

use css::domain::length::Length;
use grid::{
    GridAutoFlow, GridFr, GridGap, GridLine, GridPlacement, GridSpan, GridStyle, GridTemplateAreas,
    MaxTrackBreadth, MinTrackBreadth, TrackList, TrackSize,
};
use grid_values::{apply, inherit, reset, tokenize_value};

#[test]
fn initial_grid_style_has_standard_defaults() {
    let style = GridStyle::initial();
    assert!(style.template_columns().is_none());
    assert!(style.template_rows().is_none());
    assert_eq!(style.template_areas(), &GridTemplateAreas::none());
    assert_eq!(style.auto_columns(), TrackSize::Auto);
    assert_eq!(style.auto_rows(), TrackSize::Auto);
    assert_eq!(style.auto_flow(), GridAutoFlow::Row);
    assert!(style.column_start().is_auto());
    assert!(style.column_end().is_auto());
    assert!(style.row_start().is_auto());
    assert!(style.row_end().is_auto());
    assert_eq!(style.gap(), GridGap::ZERO);
    assert_eq!(style.row_gap(), Length::ZERO);
    assert_eq!(style.column_gap(), Length::ZERO);
}

#[test]
fn parse_grid_template_columns_track_list() {
    let tokens = tokenize_value("100px 1fr 20% auto min-content max-content");
    let tracks = grid_values::parse_track_list(&tokens).expect("should parse track list");
    assert_eq!(tracks.len(), 6);
    assert_eq!(tracks.get(0), Some(TrackSize::pixels(100.0)));
    assert_eq!(tracks.get(1), TrackSize::fr(1.0));
    assert_eq!(tracks.get(2), Some(TrackSize::percent(20.0)));
    assert_eq!(tracks.get(3), Some(TrackSize::Auto));
    assert_eq!(tracks.get(4), Some(TrackSize::MinContent));
    assert_eq!(tracks.get(5), Some(TrackSize::MaxContent));
}

#[test]
fn parse_grid_template_columns_none() {
    let tokens = tokenize_value("none");
    let tracks = grid_values::parse_track_list(&tokens).expect("none should parse");
    assert!(tracks.is_none());
    assert_eq!(tracks.len(), 0);
}

#[test]
fn parse_minmax_track_breadth() {
    let tokens = tokenize_value("minmax(100px, 1fr)");
    let track = grid_values::parse_track_size(&tokens).expect("minmax should parse");
    assert_eq!(
        track,
        TrackSize::minmax(
            MinTrackBreadth::pixels(100.0),
            MaxTrackBreadth::fr(GridFr::ONE)
        )
    );
    assert!(track.is_flexible());

    let intrinsic = tokenize_value("minmax(min-content, max-content)");
    let track_int =
        grid_values::parse_track_size(&intrinsic).expect("intrinsic minmax should parse");
    assert_eq!(
        track_int,
        TrackSize::minmax(MinTrackBreadth::MinContent, MaxTrackBreadth::MaxContent)
    );
    assert!(!track_int.is_flexible());
}

#[test]
fn minmax_rejects_flex_min_breadth() {
    // In CSS Grid L1 §7.2.1, <flex> (fr) is invalid as min in minmax.
    let tokens = tokenize_value("minmax(1fr, 100px)");
    assert!(grid_values::parse_track_size(&tokens).is_none());
}

#[test]
fn parse_repeat_function_in_track_list() {
    let tokens = tokenize_value("repeat(3, 1fr)");
    let tracks = grid_values::parse_track_list(&tokens).expect("repeat should parse");
    assert_eq!(tracks.len(), 3);
    assert_eq!(tracks.get(0), TrackSize::fr(1.0));
    assert_eq!(tracks.get(1), TrackSize::fr(1.0));
    assert_eq!(tracks.get(2), TrackSize::fr(1.0));

    let mixed = tokenize_value("100px repeat(2, 50px 1fr) auto");
    let mixed_tracks = grid_values::parse_track_list(&mixed).expect("mixed repeat should parse");
    assert_eq!(mixed_tracks.len(), 6);
    assert_eq!(mixed_tracks.get(0), Some(TrackSize::pixels(100.0)));
    assert_eq!(mixed_tracks.get(1), Some(TrackSize::pixels(50.0)));
    assert_eq!(mixed_tracks.get(2), TrackSize::fr(1.0));
    assert_eq!(mixed_tracks.get(3), Some(TrackSize::pixels(50.0)));
    assert_eq!(mixed_tracks.get(4), TrackSize::fr(1.0));
    assert_eq!(mixed_tracks.get(5), Some(TrackSize::Auto));
}

#[test]
fn parse_grid_template_areas_named_rectangles() {
    let tokens = tokenize_value("\"head head\" \"nav main\" \"foot foot\"");
    let areas = grid_values::parse_grid_template_areas(&tokens).expect("areas should parse");
    assert_eq!(areas.row_count(), 3);
    assert_eq!(areas.column_count(), 2);

    let head = areas.find_area("head").expect("head area should exist");
    assert_eq!(head.row_start(), 1);
    assert_eq!(head.row_end(), 2);
    assert_eq!(head.column_start(), 1);
    assert_eq!(head.column_end(), 3);
    assert_eq!(head.row_span(), 1);
    assert_eq!(head.column_span(), 2);

    let main = areas.find_area("main").expect("main area should exist");
    assert_eq!(main.row_start(), 2);
    assert_eq!(main.row_end(), 3);
    assert_eq!(main.column_start(), 2);
    assert_eq!(main.column_end(), 3);
}

#[test]
fn parse_grid_template_areas_with_dots_and_none() {
    let none_tokens = tokenize_value("none");
    let areas_none =
        grid_values::parse_grid_template_areas(&none_tokens).expect("none should parse");
    assert!(areas_none.is_none());

    let dotted = tokenize_value("\"a .\" \"a b\"");
    let areas_dotted =
        grid_values::parse_grid_template_areas(&dotted).expect("dotted should parse");
    let area_a = areas_dotted.find_area("a").expect("a exists");
    assert_eq!(area_a.row_start(), 1);
    assert_eq!(area_a.row_end(), 3);
    assert_eq!(area_a.column_start(), 1);
    assert_eq!(area_a.column_end(), 2);
}

#[test]
fn parse_grid_template_areas_rejects_non_rectangular_shapes() {
    // Non-rectangular L-shape area 'a'
    let l_shape = tokenize_value("\"a a\" \"a b\" \"c d\"");
    let invalid = grid_values::parse_grid_template_areas(&l_shape);
    assert!(invalid.is_none(), "L-shaped area must be rejected");

    // Unequal column counts
    let unequal = tokenize_value("\"a b c\" \"d e\"");
    assert!(grid_values::parse_grid_template_areas(&unequal).is_none());
}

#[test]
fn parse_grid_auto_flow_values() {
    let row = tokenize_value("row");
    assert_eq!(
        grid_values::parse_grid_auto_flow(&row),
        Some(GridAutoFlow::Row)
    );
    assert!(GridAutoFlow::Row.is_row());
    assert!(!GridAutoFlow::Row.is_dense());

    let col = tokenize_value("column");
    assert_eq!(
        grid_values::parse_grid_auto_flow(&col),
        Some(GridAutoFlow::Column)
    );
    assert!(GridAutoFlow::Column.is_column());

    let dense = tokenize_value("dense");
    assert_eq!(
        grid_values::parse_grid_auto_flow(&dense),
        Some(GridAutoFlow::RowDense)
    );
    assert!(GridAutoFlow::RowDense.is_dense());
    assert!(GridAutoFlow::RowDense.is_row());

    let row_dense = tokenize_value("row dense");
    assert_eq!(
        grid_values::parse_grid_auto_flow(&row_dense),
        Some(GridAutoFlow::RowDense)
    );

    let col_dense = tokenize_value("column dense");
    assert_eq!(
        grid_values::parse_grid_auto_flow(&col_dense),
        Some(GridAutoFlow::ColumnDense)
    );
    assert!(GridAutoFlow::ColumnDense.is_column());
    assert!(GridAutoFlow::ColumnDense.is_dense());
}

#[test]
fn parse_grid_line_placements() {
    let auto_tok = tokenize_value("auto");
    assert_eq!(
        grid_values::parse_grid_line_placement(&auto_tok),
        Some(GridPlacement::Auto)
    );

    let line_tok = tokenize_value("3");
    assert_eq!(
        grid_values::parse_grid_line_placement(&line_tok),
        Some(GridPlacement::Line(GridLine::new(3).unwrap()))
    );

    let neg_tok = tokenize_value("-1");
    assert_eq!(
        grid_values::parse_grid_line_placement(&neg_tok),
        Some(GridPlacement::Line(GridLine::new(-1).unwrap()))
    );

    let zero_tok = tokenize_value("0");
    assert!(
        grid_values::parse_grid_line_placement(&zero_tok).is_none(),
        "line 0 is forbidden in CSS Grid"
    );

    let span_tok = tokenize_value("span 2");
    assert_eq!(
        grid_values::parse_grid_line_placement(&span_tok),
        Some(GridPlacement::Span(GridSpan::new(2).unwrap()))
    );

    let span_zero = tokenize_value("span 0");
    assert!(
        grid_values::parse_grid_line_placement(&span_zero).is_none(),
        "span 0 is invalid in CSS Grid"
    );

    let named_tok = tokenize_value("sidebar");
    assert_eq!(
        grid_values::parse_grid_line_placement(&named_tok),
        GridPlacement::named("sidebar")
    );

    let span_named = tokenize_value("span 3 main");
    assert!(matches!(
        grid_values::parse_grid_line_placement(&span_named),
        Some(GridPlacement::SpanNamed(s, n)) if s.as_u32() == 3 && n.as_str() == "main"
    ));
}

#[test]
fn parse_grid_column_and_row_shorthand() {
    let shorthand = tokenize_value("1 / 3");
    let (start, end) =
        grid_values::parse_placement_shorthand(&shorthand).expect("shorthand parses");
    assert_eq!(start, GridPlacement::Line(GridLine::new(1).unwrap()));
    assert_eq!(end, GridPlacement::Line(GridLine::new(3).unwrap()));

    let single = tokenize_value("span 2");
    let (s_single, e_single) =
        grid_values::parse_placement_shorthand(&single).expect("single parses");
    assert_eq!(s_single, GridPlacement::Span(GridSpan::new(2).unwrap()));
    assert_eq!(e_single, GridPlacement::Auto);

    let named = tokenize_value("sidebar");
    let (s_named, e_named) = grid_values::parse_placement_shorthand(&named).expect("named parses");
    assert_eq!(s_named, GridPlacement::named("sidebar").unwrap());
    assert_eq!(e_named, GridPlacement::named("sidebar").unwrap());
}

#[test]
fn parse_grid_area_shorthand_forms() {
    let single_area = tokenize_value("header");
    let (r_s, c_s, r_e, c_e) =
        grid_values::parse_grid_area_shorthand(&single_area).expect("single area");
    let header_name = GridPlacement::named("header").unwrap();
    assert_eq!(r_s, header_name);
    assert_eq!(c_s, header_name);
    assert_eq!(r_e, header_name);
    assert_eq!(c_e, header_name);

    let four_values = tokenize_value("1 / 2 / 3 / 4");
    let (r1, c1, r2, c2) =
        grid_values::parse_grid_area_shorthand(&four_values).expect("four values");
    assert_eq!(r1, GridPlacement::Line(GridLine::new(1).unwrap()));
    assert_eq!(c1, GridPlacement::Line(GridLine::new(2).unwrap()));
    assert_eq!(r2, GridPlacement::Line(GridLine::new(3).unwrap()));
    assert_eq!(c2, GridPlacement::Line(GridLine::new(4).unwrap()));
}

#[test]
fn parse_gap_properties() {
    let normal_tok = tokenize_value("normal");
    assert_eq!(
        grid_values::parse_gap_component(&normal_tok),
        Some(Length::ZERO)
    );

    let px_tok = tokenize_value("12px");
    assert_eq!(
        grid_values::parse_gap_component(&px_tok),
        Some(Length::pixels(12.0))
    );

    let uniform_gap = tokenize_value("16px");
    let gap_uni = grid_values::parse_gap_shorthand(&uniform_gap).expect("uniform gap parses");
    assert_eq!(gap_uni, GridGap::uniform(Length::pixels(16.0)));

    let two_gaps = tokenize_value("10px 20px");
    let gap_two = grid_values::parse_gap_shorthand(&two_gaps).expect("two gaps parse");
    assert_eq!(gap_two.row(), Length::pixels(10.0));
    assert_eq!(gap_two.column(), Length::pixels(20.0));
}

#[test]
fn cascade_apply_and_reset_properties() {
    let initial = GridStyle::initial();

    // 1. grid-template-columns
    let cols_tok = tokenize_value("100px 1fr");
    let s1 = apply(initial, "grid-template-columns", &cols_tok).expect("apply template-columns");
    assert_eq!(s1.template_columns().len(), 2);

    // 2. grid-template-rows
    let rows_tok = tokenize_value("50px 2fr");
    let s2 = apply(s1, "grid-template-rows", &rows_tok).expect("apply template-rows");
    assert_eq!(s2.template_rows().len(), 2);

    // 3. grid-auto-columns & grid-auto-rows
    let auto_col = tokenize_value("120px");
    let s3 = apply(s2, "grid-auto-columns", &auto_col).expect("apply auto-columns");
    assert_eq!(s3.auto_columns(), TrackSize::pixels(120.0));

    let auto_row = tokenize_value("min-content");
    let s4 = apply(s3, "grid-auto-rows", &auto_row).expect("apply auto-rows");
    assert_eq!(s4.auto_rows(), TrackSize::MinContent);

    // 4. grid-auto-flow
    let flow_tok = tokenize_value("column dense");
    let s5 = apply(s4, "grid-auto-flow", &flow_tok).expect("apply auto-flow");
    assert_eq!(s5.auto_flow(), GridAutoFlow::ColumnDense);

    // 5. grid-column and grid-row
    let axis_placement_tok = tokenize_value("2 / span 3");
    let s6 = apply(s5, "grid-column", &axis_placement_tok).expect("apply grid-column");
    assert_eq!(
        s6.column_start(),
        &GridPlacement::Line(GridLine::new(2).unwrap())
    );
    assert_eq!(
        s6.column_end(),
        &GridPlacement::Span(GridSpan::new(3).unwrap())
    );

    // 6. gap
    let gap_tok = tokenize_value("8px 16px");
    let s7 = apply(s6, "gap", &gap_tok).expect("apply gap");
    assert_eq!(s7.row_gap(), Length::pixels(8.0));
    assert_eq!(s7.column_gap(), Length::pixels(16.0));

    // Reset gap
    let s8 = reset(s7, "gap").expect("reset gap");
    assert_eq!(s8.gap(), GridGap::ZERO);

    // Inherit gap from parent
    let s9 = inherit(s8, &s7, "gap").expect("inherit gap");
    assert_eq!(s9.gap(), s7.gap());
}

#[test]
fn display_implementations_are_clean() {
    let fr = GridFr::new(2.5).unwrap();
    assert_eq!(format!("{fr}"), "2.5fr");

    let track = TrackSize::minmax(MinTrackBreadth::pixels(100.0), MaxTrackBreadth::fr(fr));
    assert_eq!(format!("{track}"), "minmax(100px, 2.5fr)");

    let track_list = TrackList::from_tracks(&[TrackSize::pixels(50.0), TrackSize::Auto]);
    assert_eq!(format!("{track_list}"), "50px auto");

    let gap = GridGap::new(Length::pixels(10.0), Length::pixels(20.0));
    assert_eq!(format!("{gap}"), "10px 20px");
}
