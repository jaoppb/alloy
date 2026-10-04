//! Parsing and cascade resolution for CSS Grid Layout Level 1/2 properties.

use crate::domain::computed::grid::{
    GridAreaName, GridAutoFlow, GridFr, GridGap, GridLine, GridLineName, GridPlacement, GridSpan,
    GridStyle, GridTemplateAreas, MaxTrackBreadth, MinTrackBreadth, TrackList, TrackSize,
};
use crate::domain::length::Length;
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::values::{
    component_values, function_call, length_from_token, significant_tokens, split_top_level_commas,
};

/// Tokenizes a CSS value string and filters out whitespace — the cascade's own
/// tokenizer path, exposed for callers that hold raw value text.
#[must_use]
pub fn tokenize_value(value_text: &str) -> Vec<Token> {
    significant_tokens(value_text)
}

/// Applies a CSS Grid property declaration to [`GridStyle`].
#[must_use]
pub fn apply(grid: &GridStyle, property: &str, tokens: &[Token]) -> Option<GridStyle> {
    match property {
        "grid-template-columns" => {
            parse_track_list(tokens).map(|tracks| grid.with_template_columns(tracks))
        }
        "grid-template-rows" => {
            parse_track_list(tokens).map(|tracks| grid.with_template_rows(tracks))
        }
        "grid-template-areas" => {
            parse_grid_template_areas(tokens).map(|areas| grid.with_template_areas(areas))
        }
        "grid-auto-columns" => parse_track_size(tokens).map(|size| grid.with_auto_columns(size)),
        "grid-auto-rows" => parse_track_size(tokens).map(|size| grid.with_auto_rows(size)),
        "grid-auto-flow" => parse_grid_auto_flow(tokens).map(|flow| grid.with_auto_flow(flow)),
        "grid-column-start" => {
            parse_grid_line_placement(tokens).map(|placement| grid.with_column_start(placement))
        }
        "grid-column-end" => {
            parse_grid_line_placement(tokens).map(|placement| grid.with_column_end(placement))
        }
        "grid-row-start" => {
            parse_grid_line_placement(tokens).map(|placement| grid.with_row_start(placement))
        }
        "grid-row-end" => {
            parse_grid_line_placement(tokens).map(|placement| grid.with_row_end(placement))
        }
        "grid-column" => apply_column_shorthand(grid, tokens),
        "grid-row" => apply_row_shorthand(grid, tokens),
        "grid-area" => apply_area_shorthand(grid, tokens),
        "row-gap" | "grid-row-gap" => parse_gap_component(tokens).map(|gap| grid.with_row_gap(gap)),
        "column-gap" | "grid-column-gap" => {
            parse_gap_component(tokens).map(|gap| grid.with_column_gap(gap))
        }
        "gap" | "grid-gap" => parse_gap_shorthand(tokens).map(|gap| grid.with_gap(gap)),
        _ => None,
    }
}

fn apply_column_shorthand(grid: &GridStyle, tokens: &[Token]) -> Option<GridStyle> {
    let (start, end) = parse_placement_shorthand(tokens)?;
    Some(grid.with_column_start(start).with_column_end(end))
}

fn apply_row_shorthand(grid: &GridStyle, tokens: &[Token]) -> Option<GridStyle> {
    let (start, end) = parse_placement_shorthand(tokens)?;
    Some(grid.with_row_start(start).with_row_end(end))
}

fn apply_area_shorthand(grid: &GridStyle, tokens: &[Token]) -> Option<GridStyle> {
    let (row_start, column_start, row_end, column_end) = parse_grid_area_shorthand(tokens)?;
    Some(
        grid.with_row_start(row_start)
            .with_column_start(column_start)
            .with_row_end(row_end)
            .with_column_end(column_end),
    )
}

/// Resets a CSS Grid property to its initial value.
#[must_use]
pub fn reset(grid: &GridStyle, property: &str) -> Option<GridStyle> {
    copy_property(grid, &GridStyle::initial(), property)
}

/// Inherits a CSS Grid property from a parent [`GridStyle`].
#[must_use]
pub fn inherit(grid: &GridStyle, parent: &GridStyle, property: &str) -> Option<GridStyle> {
    copy_property(grid, parent, property)
}

fn copy_property(grid: &GridStyle, source: &GridStyle, property: &str) -> Option<GridStyle> {
    match property {
        "grid-template-columns" => Some(grid.with_template_columns(*source.template_columns())),
        "grid-template-rows" => Some(grid.with_template_rows(*source.template_rows())),
        "grid-template-areas" => Some(grid.with_template_areas(*source.template_areas())),
        "grid-auto-columns" => Some(grid.with_auto_columns(source.auto_columns())),
        "grid-auto-rows" => Some(grid.with_auto_rows(source.auto_rows())),
        "grid-auto-flow" => Some(grid.with_auto_flow(source.auto_flow())),
        "grid-column-start" => Some(grid.with_column_start(*source.column_start())),
        "grid-column-end" => Some(grid.with_column_end(*source.column_end())),
        "grid-row-start" => Some(grid.with_row_start(*source.row_start())),
        "grid-row-end" => Some(grid.with_row_end(*source.row_end())),
        "grid-column" => Some(copy_column(grid, source)),
        "grid-row" => Some(copy_row(grid, source)),
        "grid-area" => Some(copy_area(grid, source)),
        "row-gap" | "grid-row-gap" => Some(grid.with_row_gap(source.row_gap())),
        "column-gap" | "grid-column-gap" => Some(grid.with_column_gap(source.column_gap())),
        "gap" | "grid-gap" => Some(grid.with_gap(source.gap())),
        _ => None,
    }
}

const fn copy_column(grid: &GridStyle, source: &GridStyle) -> GridStyle {
    grid.with_column_start(*source.column_start())
        .with_column_end(*source.column_end())
}

const fn copy_row(grid: &GridStyle, source: &GridStyle) -> GridStyle {
    grid.with_row_start(*source.row_start())
        .with_row_end(*source.row_end())
}

const fn copy_area(grid: &GridStyle, source: &GridStyle) -> GridStyle {
    grid.with_row_start(*source.row_start())
        .with_column_start(*source.column_start())
        .with_row_end(*source.row_end())
        .with_column_end(*source.column_end())
}

/// Parses a track size (`Length`, percentage, `fr`, `auto`, `min-content`, `max-content`, or `minmax`).
#[must_use]
pub fn parse_track_size(tokens: &[Token]) -> Option<TrackSize> {
    match tokens {
        [Token::Function(name), ..] if name.eq_ignore_ascii_case("minmax") => parse_minmax(tokens),
        [Token::Ident(keyword)] => parse_track_keyword(keyword),
        [Token::Dimension(magnitude, unit)] if unit.eq_ignore_ascii_case("fr") => {
            TrackSize::fr(*magnitude)
        }
        [single] => length_from_token(single).map(TrackSize::Length),
        _ => None,
    }
}

fn parse_track_keyword(keyword: &str) -> Option<TrackSize> {
    match keyword.to_ascii_lowercase().as_str() {
        "auto" => Some(TrackSize::Auto),
        "min-content" => Some(TrackSize::MinContent),
        "max-content" => Some(TrackSize::MaxContent),
        _ => None,
    }
}

/// Parses a `minmax(min, max)` functional notation.
#[must_use]
pub fn parse_minmax(tokens: &[Token]) -> Option<TrackSize> {
    let arguments = extract_function_arguments(tokens, "minmax")?;
    let parts = split_top_level_commas(arguments);
    let [min_tokens, max_tokens] = parts.as_slice() else {
        return None;
    };
    let min = parse_min_breadth(min_tokens)?;
    let max = parse_max_breadth(max_tokens)?;
    Some(TrackSize::MinMax(min, max))
}

/// The arguments of `tokens` when it is exactly one call to `expected_name`
/// (function names are ASCII case-insensitive, CSS Syntax 3 §4.3.4).
fn extract_function_arguments<'a>(tokens: &'a [Token], expected_name: &str) -> Option<&'a [Token]> {
    let (name, arguments) = function_call(tokens)?;
    name.eq_ignore_ascii_case(expected_name)
        .then_some(arguments)
}

fn parse_min_breadth(tokens: &[Token]) -> Option<MinTrackBreadth> {
    match tokens {
        [Token::Ident(keyword)] => parse_min_breadth_keyword(keyword),
        [single] => length_from_token(single).map(MinTrackBreadth::Length),
        _ => None,
    }
}

fn parse_min_breadth_keyword(keyword: &str) -> Option<MinTrackBreadth> {
    match keyword.to_ascii_lowercase().as_str() {
        "auto" => Some(MinTrackBreadth::Auto),
        "min-content" => Some(MinTrackBreadth::MinContent),
        "max-content" => Some(MinTrackBreadth::MaxContent),
        _ => None,
    }
}

fn parse_max_breadth(tokens: &[Token]) -> Option<MaxTrackBreadth> {
    match tokens {
        [Token::Ident(keyword)] => parse_max_breadth_keyword(keyword),
        [Token::Dimension(magnitude, unit)] if unit.eq_ignore_ascii_case("fr") => {
            GridFr::new(*magnitude).map(MaxTrackBreadth::Flex)
        }
        [single] => length_from_token(single).map(MaxTrackBreadth::Length),
        _ => None,
    }
}

fn parse_max_breadth_keyword(keyword: &str) -> Option<MaxTrackBreadth> {
    match keyword.to_ascii_lowercase().as_str() {
        "auto" => Some(MaxTrackBreadth::Auto),
        "min-content" => Some(MaxTrackBreadth::MinContent),
        "max-content" => Some(MaxTrackBreadth::MaxContent),
        _ => None,
    }
}

/// Parses an explicit track list (`grid-template-columns`, `grid-template-rows`).
#[must_use]
pub fn parse_track_list(tokens: &[Token]) -> Option<TrackList> {
    if matches!(tokens, [Token::Ident(name)] if name.eq_ignore_ascii_case("none")) {
        return Some(TrackList::none());
    }
    parse_track_sequence(tokens)
}

/// A whitespace-separated sequence of track sizes and `repeat()` calls,
/// expanded. More than [`TrackList::CAPACITY`] tracks is a refusal (with a
/// warning), never a silent truncation — see `tests/data/MANIFEST.md`'s
/// "Grid storage caps".
fn parse_track_sequence(tokens: &[Token]) -> Option<TrackList> {
    let tracks = component_values(tokens).try_fold(Vec::new(), append_track_component)?;
    if tracks.is_empty() {
        return None;
    }
    TrackList::from_tracks(&tracks)
}

/// `tracks` with the tracks `component` expands to appended, or `None` when the
/// component is invalid or the total would exceed [`TrackList::CAPACITY`].
fn append_track_component(
    mut tracks: Vec<TrackSize>,
    component: &[Token],
) -> Option<Vec<TrackSize>> {
    let expanded = expand_track_component(component)?;
    let total = tracks.len().saturating_add(expanded.len());
    if total > TrackList::CAPACITY {
        warn_track_capacity(total);
        return None;
    }
    tracks.extend(expanded);
    Some(tracks)
}

fn expand_track_component(component: &[Token]) -> Option<Vec<TrackSize>> {
    if is_repeat_call(component) {
        return parse_repeat(component);
    }
    parse_track_size(component).map(|track| vec![track])
}

fn is_repeat_call(component: &[Token]) -> bool {
    matches!(component, [Token::Function(name), ..] if name.eq_ignore_ascii_case("repeat"))
}

/// `repeat(<count>, <track-size>+)` (CSS Grid L1 §7.2.3). The count is
/// author-controlled, so the expanded length is checked against
/// [`TrackList::CAPACITY`] **before** anything is allocated: `repeat(4000000000,
/// 1fr 1fr)` must cost a multiplication, not eight billion pushes.
fn parse_repeat(tokens: &[Token]) -> Option<Vec<TrackSize>> {
    let arguments = extract_function_arguments(tokens, "repeat")?;
    let parts = split_top_level_commas(arguments);
    let [count_tokens, pattern_tokens] = parts.as_slice() else {
        return None;
    };
    let count = parse_repeat_count(count_tokens)?;
    let pattern = parse_repeat_pattern(pattern_tokens)?;
    let total = count
        .checked_mul(pattern.len())
        .filter(|total| *total <= TrackList::CAPACITY);
    let Some(total) = total else {
        warn_track_capacity(count.saturating_mul(pattern.len()));
        return None;
    };
    Some(pattern.iter().copied().cycle().take(total).collect())
}

/// The track sizes a `repeat()` repeats. A nested `repeat()` is refused — CSS
/// Grid L1 §7.2.3's `<track-repeat>` takes `<track-size>`s only — which also
/// keeps the expansion one multiplication deep.
fn parse_repeat_pattern(tokens: &[Token]) -> Option<Vec<TrackSize>> {
    let pattern: Option<Vec<TrackSize>> =
        component_values(tokens).map(parse_repeated_track).collect();
    pattern.filter(|tracks| !tracks.is_empty())
}

fn parse_repeated_track(component: &[Token]) -> Option<TrackSize> {
    if is_repeat_call(component) {
        tracing::warn!("nested repeat() in a grid track list is invalid (CSS Grid L1 §7.2.3)");
        return None;
    }
    parse_track_size(component)
}

/// The repeat count: a positive integer. A fractional or non-positive number
/// fails the integer parse or the filter.
fn parse_repeat_count(tokens: &[Token]) -> Option<usize> {
    let [Token::Number(magnitude)] = tokens else {
        return None;
    };
    format!("{magnitude}")
        .parse::<usize>()
        .ok()
        .filter(|count| *count >= 1)
}

fn warn_track_capacity(requested: usize) {
    tracing::warn!(
        requested,
        capacity = TrackList::CAPACITY,
        "grid track list exceeds the fixed track capacity; declaration rejected"
    );
}

/// Parses `grid-template-areas`.
#[must_use]
pub fn parse_grid_template_areas(tokens: &[Token]) -> Option<GridTemplateAreas> {
    if matches!(tokens, [Token::Ident(name)] if name.eq_ignore_ascii_case("none")) {
        return Some(GridTemplateAreas::none());
    }
    parse_area_rows(tokens)
}

/// One row per string. More than [`GridTemplateAreas::CAPACITY`] cells is a
/// refusal with a warning — the domain type would refuse it silently.
fn parse_area_rows(tokens: &[Token]) -> Option<GridTemplateAreas> {
    let matrix: Option<Vec<Vec<Option<GridAreaName>>>> =
        tokens.iter().map(parse_area_string_token).collect();
    let matrix = matrix?;
    let cell_count = matrix
        .iter()
        .map(Vec::len)
        .fold(0_usize, usize::saturating_add);
    if cell_count > GridTemplateAreas::CAPACITY {
        tracing::warn!(
            cells = cell_count,
            capacity = GridTemplateAreas::CAPACITY,
            "grid-template-areas exceeds the fixed cell capacity; declaration rejected"
        );
        return None;
    }
    GridTemplateAreas::from_matrix(&matrix)
}

/// One cell of a `grid-template-areas` row (CSS Grid L1 §7.3).
enum AreaCell {
    /// A run of one or more `.` — the null cell token.
    Empty,
    Named(GridAreaName),
}

impl AreaCell {
    const fn into_name(self) -> Option<GridAreaName> {
        match self {
            Self::Empty => None,
            Self::Named(name) => Some(name),
        }
    }
}

fn parse_area_string_token(token: &Token) -> Option<Vec<Option<GridAreaName>>> {
    let Token::QuotedString(row_text) = token else {
        return None;
    };
    let row: Option<Vec<Option<GridAreaName>>> = row_text
        .split_whitespace()
        .map(|cell| parse_area_cell(cell).map(AreaCell::into_name))
        .collect();
    row.filter(|cells| !cells.is_empty())
}

fn parse_area_cell(cell: &str) -> Option<AreaCell> {
    if cell.chars().all(|character| character == '.') {
        return Some(AreaCell::Empty);
    }
    area_name(cell).map(AreaCell::Named)
}

/// A named area, refused with a warning past [`GridAreaName::CAPACITY`] bytes.
fn area_name(cell: &str) -> Option<GridAreaName> {
    if cell.len() > GridAreaName::CAPACITY {
        warn_name_capacity(cell.len(), GridAreaName::CAPACITY);
        return None;
    }
    GridAreaName::new(cell)
}

/// A named line, refused with a warning past [`GridLineName::CAPACITY`] bytes.
fn line_name(name: &str) -> Option<GridLineName> {
    if name.len() > GridLineName::CAPACITY {
        warn_name_capacity(name.len(), GridLineName::CAPACITY);
        return None;
    }
    GridLineName::new(name)
}

fn warn_name_capacity(length: usize, capacity: usize) {
    tracing::warn!(
        length,
        capacity,
        "grid name exceeds the fixed name capacity; declaration rejected"
    );
}

/// Parses `grid-auto-flow`.
#[must_use]
pub fn parse_grid_auto_flow(tokens: &[Token]) -> Option<GridAutoFlow> {
    match tokens {
        [Token::Ident(keyword)] => parse_single_flow_keyword(keyword),
        [Token::Ident(first), Token::Ident(second)] => parse_pair_flow_keywords(first, second),
        _ => None,
    }
}

fn parse_single_flow_keyword(keyword: &str) -> Option<GridAutoFlow> {
    match keyword.to_ascii_lowercase().as_str() {
        "row" => Some(GridAutoFlow::Row),
        "column" => Some(GridAutoFlow::Column),
        "dense" => Some(GridAutoFlow::RowDense),
        _ => None,
    }
}

fn parse_pair_flow_keywords(first: &str, second: &str) -> Option<GridAutoFlow> {
    let first_keyword = first.to_ascii_lowercase();
    let second_keyword = second.to_ascii_lowercase();
    match (first_keyword.as_str(), second_keyword.as_str()) {
        ("row", "dense") | ("dense", "row") => Some(GridAutoFlow::RowDense),
        ("column", "dense") | ("dense", "column") => Some(GridAutoFlow::ColumnDense),
        _ => None,
    }
}

/// Parses item placement on a single axis (`grid-column-start`, `grid-row-end`, etc.).
#[must_use]
pub fn parse_grid_line_placement(tokens: &[Token]) -> Option<GridPlacement> {
    match tokens {
        [Token::Ident(name)] => parse_placement_ident(name),
        [Token::Number(number)] => parse_signed_line_number(*number).map(GridPlacement::Line),
        [Token::Ident(span_keyword), Token::Number(number)] if is_span(span_keyword) => {
            parse_positive_span(*number).map(GridPlacement::Span)
        }
        [Token::Ident(span_keyword), Token::Ident(name)] if is_span(span_keyword) => {
            line_name(name).map(|target| GridPlacement::SpanNamed(GridSpan::ONE, target))
        }
        [
            Token::Ident(span_keyword),
            Token::Number(number),
            Token::Ident(name),
        ] if is_span(span_keyword) => {
            let span = parse_positive_span(*number)?;
            Some(GridPlacement::SpanNamed(span, line_name(name)?))
        }
        [Token::Number(number), Token::Ident(name)] => {
            let line = parse_signed_line_number(*number)?;
            Some(GridPlacement::LineNamed(line, line_name(name)?))
        }
        _ => None,
    }
}

const fn is_span(keyword: &str) -> bool {
    keyword.eq_ignore_ascii_case("span")
}

fn parse_placement_ident(ident: &str) -> Option<GridPlacement> {
    if ident.eq_ignore_ascii_case("auto") {
        return Some(GridPlacement::Auto);
    }
    line_name(ident).map(GridPlacement::Named)
}

fn parse_signed_line_number(number: f32) -> Option<GridLine> {
    format!("{number}")
        .parse::<i32>()
        .ok()
        .and_then(GridLine::new)
}

fn parse_positive_span(number: f32) -> Option<GridSpan> {
    format!("{number}")
        .parse::<u32>()
        .ok()
        .and_then(GridSpan::new)
}

/// `tokens` split at every `/` — the separator of the placement shorthands
/// (CSS Grid L1 §8.4).
fn split_at_slashes(tokens: &[Token]) -> Vec<&[Token]> {
    tokens
        .split(|token| matches!(token, Token::Delimiter('/')))
        .collect()
}

/// Parses placement shorthand for one axis (`grid-column: start / end`).
#[must_use]
pub fn parse_placement_shorthand(tokens: &[Token]) -> Option<(GridPlacement, GridPlacement)> {
    let parts = split_at_slashes(tokens);
    match parts.as_slice() {
        [single] => {
            let start = parse_grid_line_placement(single)?;
            let end = default_placement_end(&start);
            Some((start, end))
        }
        [start_tokens, end_tokens] => {
            let start = parse_grid_line_placement(start_tokens)?;
            let end = parse_grid_line_placement(end_tokens)?;
            Some((start, end))
        }
        _ => None,
    }
}

const fn default_placement_end(start: &GridPlacement) -> GridPlacement {
    match start {
        GridPlacement::Named(name) => GridPlacement::Named(*name),
        _ => GridPlacement::Auto,
    }
}

/// Parses `grid-area` shorthand (`row-start / col-start / row-end / col-end`).
///
/// An omitted end copies its start when that start is a custom-ident, and is
/// `auto` otherwise (CSS Grid L1 §8.4).
#[must_use]
pub fn parse_grid_area_shorthand(
    tokens: &[Token],
) -> Option<(GridPlacement, GridPlacement, GridPlacement, GridPlacement)> {
    let parts = split_at_slashes(tokens);
    match parts.as_slice() {
        [single] => expand_single_area(single),
        [row_start, column_start] => {
            let row_start = parse_grid_line_placement(row_start)?;
            let column_start = parse_grid_line_placement(column_start)?;
            let row_end = default_placement_end(&row_start);
            let column_end = default_placement_end(&column_start);
            Some((row_start, column_start, row_end, column_end))
        }
        [row_start, column_start, row_end] => {
            let column_start = parse_grid_line_placement(column_start)?;
            let column_end = default_placement_end(&column_start);
            Some((
                parse_grid_line_placement(row_start)?,
                column_start,
                parse_grid_line_placement(row_end)?,
                column_end,
            ))
        }
        [row_start, column_start, row_end, column_end] => Some((
            parse_grid_line_placement(row_start)?,
            parse_grid_line_placement(column_start)?,
            parse_grid_line_placement(row_end)?,
            parse_grid_line_placement(column_end)?,
        )),
        _ => None,
    }
}

fn expand_single_area(
    tokens: &[Token],
) -> Option<(GridPlacement, GridPlacement, GridPlacement, GridPlacement)> {
    let placement = parse_grid_line_placement(tokens)?;
    match placement {
        GridPlacement::Named(name) => Some((
            GridPlacement::Named(name),
            GridPlacement::Named(name),
            GridPlacement::Named(name),
            GridPlacement::Named(name),
        )),
        other => Some((
            other,
            GridPlacement::Auto,
            GridPlacement::Auto,
            GridPlacement::Auto,
        )),
    }
}

/// Parses a single gap length (`row-gap`, `column-gap`).
#[must_use]
pub fn parse_gap_component(tokens: &[Token]) -> Option<Length> {
    match tokens {
        [Token::Ident(name)] if name.eq_ignore_ascii_case("normal") => Some(Length::ZERO),
        [single] => length_from_token(single),
        _ => None,
    }
}

/// Parses gap shorthand (`gap: <row-gap> <column-gap>?`).
#[must_use]
pub fn parse_gap_shorthand(tokens: &[Token]) -> Option<GridGap> {
    match tokens {
        [single] => {
            let length = parse_gap_component(core::slice::from_ref(single))?;
            Some(GridGap::uniform(length))
        }
        [first, second] => {
            let row = parse_gap_component(core::slice::from_ref(first))?;
            let column = parse_gap_component(core::slice::from_ref(second))?;
            Some(GridGap::new(row, column))
        }
        _ => None,
    }
}
