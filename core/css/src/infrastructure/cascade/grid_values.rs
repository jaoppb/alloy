//! Parsing and cascade resolution for CSS Grid Layout Level 1/2 properties.

use crate::domain::computed::grid::{
    GridAreaName, GridAutoFlow, GridFr, GridGap, GridLine, GridLineName, GridPlacement, GridSpan,
    GridStyle, GridTemplateAreas, MaxTrackBreadth, MinTrackBreadth, TrackList, TrackSize,
};
use crate::domain::length::Length;
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::tokenizer::tokenize;

/// Tokenizes a CSS value string and filters out whitespace.
#[must_use]
pub fn tokenize_value(value_str: &str) -> Vec<Token> {
    tokenize(value_str)
        .iter()
        .map(|spanned| spanned.token().clone())
        .filter(|token| !token.is_whitespace())
        .collect()
}

/// Applies a CSS Grid property declaration to [`GridStyle`].
#[must_use]
pub fn apply(grid: GridStyle, property: &str, tokens: &[Token]) -> Option<GridStyle> {
    match property {
        "grid-template-columns" => parse_track_list(tokens).map(|t| grid.with_template_columns(t)),
        "grid-template-rows" => parse_track_list(tokens).map(|t| grid.with_template_rows(t)),
        "grid-template-areas" => {
            parse_grid_template_areas(tokens).map(|a| grid.with_template_areas(a))
        }
        "grid-auto-columns" => parse_track_size(tokens).map(|s| grid.with_auto_columns(s)),
        "grid-auto-rows" => parse_track_size(tokens).map(|s| grid.with_auto_rows(s)),
        "grid-auto-flow" => parse_grid_auto_flow(tokens).map(|f| grid.with_auto_flow(f)),
        "grid-column-start" => parse_grid_line_placement(tokens).map(|p| grid.with_column_start(p)),
        "grid-column-end" => parse_grid_line_placement(tokens).map(|p| grid.with_column_end(p)),
        "grid-row-start" => parse_grid_line_placement(tokens).map(|p| grid.with_row_start(p)),
        "grid-row-end" => parse_grid_line_placement(tokens).map(|p| grid.with_row_end(p)),
        "grid-column" => apply_column_shorthand(grid, tokens),
        "grid-row" => apply_row_shorthand(grid, tokens),
        "grid-area" => apply_area_shorthand(grid, tokens),
        "row-gap" | "grid-row-gap" => parse_gap_component(tokens).map(|g| grid.with_row_gap(g)),
        "column-gap" | "grid-column-gap" => {
            parse_gap_component(tokens).map(|g| grid.with_column_gap(g))
        }
        "gap" | "grid-gap" => parse_gap_shorthand(tokens).map(|g| grid.with_gap(g)),
        _ => None,
    }
}

fn apply_column_shorthand(grid: GridStyle, tokens: &[Token]) -> Option<GridStyle> {
    let (start, end) = parse_placement_shorthand(tokens)?;
    Some(grid.with_column_start(start).with_column_end(end))
}

fn apply_row_shorthand(grid: GridStyle, tokens: &[Token]) -> Option<GridStyle> {
    let (start, end) = parse_placement_shorthand(tokens)?;
    Some(grid.with_row_start(start).with_row_end(end))
}

fn apply_area_shorthand(grid: GridStyle, tokens: &[Token]) -> Option<GridStyle> {
    let (r_s, c_s, r_e, c_e) = parse_grid_area_shorthand(tokens)?;
    Some(
        grid.with_row_start(r_s)
            .with_column_start(c_s)
            .with_row_end(r_e)
            .with_column_end(c_e),
    )
}

/// Resets a CSS Grid property to its initial value.
#[must_use]
pub fn reset(grid: GridStyle, property: &str) -> Option<GridStyle> {
    copy_property(grid, &GridStyle::initial(), property)
}

/// Inherits a CSS Grid property from a parent [`GridStyle`].
#[must_use]
pub fn inherit(grid: GridStyle, parent: &GridStyle, property: &str) -> Option<GridStyle> {
    copy_property(grid, parent, property)
}

fn copy_property(grid: GridStyle, source: &GridStyle, property: &str) -> Option<GridStyle> {
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

const fn copy_column(grid: GridStyle, source: &GridStyle) -> GridStyle {
    grid.with_column_start(*source.column_start())
        .with_column_end(*source.column_end())
}

const fn copy_row(grid: GridStyle, source: &GridStyle) -> GridStyle {
    grid.with_row_start(*source.row_start())
        .with_row_end(*source.row_end())
}

const fn copy_area(grid: GridStyle, source: &GridStyle) -> GridStyle {
    grid.with_row_start(*source.row_start())
        .with_column_start(*source.column_start())
        .with_row_end(*source.row_end())
        .with_column_end(*source.column_end())
}

/// Parses a length token into [`Length`].
#[must_use]
pub fn length_from_token(token: &Token) -> Option<Length> {
    match token {
        Token::Dimension(magnitude, unit) => length_with_unit(*magnitude, unit),
        Token::Percentage(magnitude) => Some(Length::Percent(*magnitude)),
        Token::Number(magnitude) => zero_length_number(*magnitude),
        _ => None,
    }
}

fn zero_length_number(magnitude: f32) -> Option<Length> {
    if magnitude == 0.0 {
        return Some(Length::ZERO);
    }
    None
}

fn length_with_unit(magnitude: f32, unit: &str) -> Option<Length> {
    match unit.to_ascii_lowercase().as_str() {
        "px" => Some(Length::Pixels(magnitude)),
        "em" => Some(Length::Em(magnitude)),
        "rem" => Some(Length::Rem(magnitude)),
        "pt" => Some(Length::Points(magnitude)),
        _ => None,
    }
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
    let parts: Vec<&[Token]> = arguments.split(|t| matches!(t, Token::Comma)).collect();
    let [min_tokens, max_tokens] = parts.as_slice() else {
        return None;
    };
    let min = parse_min_breadth(min_tokens)?;
    let max = parse_max_breadth(max_tokens)?;
    Some(TrackSize::MinMax(min, max))
}

fn extract_function_arguments<'a>(tokens: &'a [Token], expected_name: &str) -> Option<&'a [Token]> {
    let [
        Token::Function(name),
        arguments @ ..,
        Token::CloseParenthesis,
    ] = tokens
    else {
        return None;
    };
    if !name.eq_ignore_ascii_case(expected_name) {
        return None;
    }
    Some(arguments)
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

fn parse_track_sequence(tokens: &[Token]) -> Option<TrackList> {
    let mut tracks = Vec::new();
    let mut index: usize = 0;
    while index < tokens.len() {
        let chunk_slice = tokens.get(index..)?;
        let span = token_run_span(chunk_slice);
        let end = index.saturating_add(span);
        let chunk = tokens.get(index..end)?;
        consume_track_chunk(&mut tracks, chunk)?;
        index = end;
    }
    if tracks.is_empty() {
        return None;
    }
    Some(TrackList::from_tracks(&tracks))
}

fn consume_track_chunk(tracks: &mut Vec<TrackSize>, chunk: &[Token]) -> Option<()> {
    match chunk {
        [Token::Function(name), ..] if name.eq_ignore_ascii_case("repeat") => {
            let repeated = parse_repeat(chunk)?;
            tracks.extend(repeated);
            Some(())
        }
        _ => {
            let track = parse_track_size(chunk)?;
            tracks.push(track);
            Some(())
        }
    }
}

fn parse_repeat(tokens: &[Token]) -> Option<Vec<TrackSize>> {
    let arguments = extract_function_arguments(tokens, "repeat")?;
    let comma_pos = arguments.iter().position(|t| matches!(t, Token::Comma))?;
    let count_tokens = arguments.get(..comma_pos)?;
    let track_tokens = arguments.get(comma_pos.saturating_add(1)..)?;
    let count = parse_positive_int(count_tokens)?;
    let count_usize = usize::try_from(count).ok()?;
    let sub_list = parse_track_sequence(track_tokens)?;
    let total_len = sub_list.len().saturating_mul(count_usize);
    let mut result = Vec::with_capacity(total_len);
    for _ in 0..count_usize {
        result.extend(sub_list.tracks().iter().copied());
    }
    Some(result)
}

fn parse_positive_int(tokens: &[Token]) -> Option<u32> {
    let [Token::Number(magnitude)] = tokens else {
        return None;
    };
    format!("{magnitude}")
        .parse::<u32>()
        .ok()
        .filter(|&v| v >= 1)
}

fn token_run_span(tokens: &[Token]) -> usize {
    match tokens.first() {
        Some(Token::Function(_)) => function_span(tokens),
        _ => 1,
    }
}

fn function_span(tokens: &[Token]) -> usize {
    let mut depth: usize = 0;
    for (index, token) in tokens.iter().enumerate() {
        depth = paren_depth(depth, token);
        if depth == 0 {
            return index.saturating_add(1);
        }
    }
    tokens.len()
}

const fn paren_depth(depth: usize, token: &Token) -> usize {
    match token {
        Token::Function(_) | Token::OpenParenthesis => depth.saturating_add(1),
        Token::CloseParenthesis => depth.saturating_sub(1),
        _ => depth,
    }
}

/// Parses `grid-template-areas`.
#[must_use]
pub fn parse_grid_template_areas(tokens: &[Token]) -> Option<GridTemplateAreas> {
    if matches!(tokens, [Token::Ident(name)] if name.eq_ignore_ascii_case("none")) {
        return Some(GridTemplateAreas::none());
    }
    parse_area_rows(tokens)
}

fn parse_area_rows(tokens: &[Token]) -> Option<GridTemplateAreas> {
    let mut matrix = Vec::new();
    for token in tokens {
        let row_matrix = parse_area_string_token(token)?;
        matrix.push(row_matrix);
    }
    GridTemplateAreas::from_matrix(&matrix)
}

enum AreaCellToken {
    Empty,
    Named(GridAreaName),
}

fn parse_area_string_token(token: &Token) -> Option<Vec<Option<GridAreaName>>> {
    let Token::QuotedString(row_text) = token else {
        return None;
    };
    let cells = row_text
        .split_whitespace()
        .map(parse_area_cell)
        .collect::<Option<Vec<_>>>()?;
    if cells.is_empty() {
        return None;
    }
    let row = cells
        .into_iter()
        .map(|c| match c {
            AreaCellToken::Empty => None,
            AreaCellToken::Named(name) => Some(name),
        })
        .collect();
    Some(row)
}

fn parse_area_cell(cell: &str) -> Option<AreaCellToken> {
    if cell == "." || cell.chars().all(|c| c == '.') {
        return Some(AreaCellToken::Empty);
    }
    GridAreaName::new(cell).map(AreaCellToken::Named)
}

/// Parses `grid-auto-flow`.
#[must_use]
pub fn parse_grid_auto_flow(tokens: &[Token]) -> Option<GridAutoFlow> {
    match tokens {
        [Token::Ident(a)] => parse_single_flow_ident(a),
        [Token::Ident(a), Token::Ident(b)] => parse_pair_flow_ident(a, b),
        _ => None,
    }
}

fn parse_single_flow_ident(ident: &str) -> Option<GridAutoFlow> {
    match ident.to_ascii_lowercase().as_str() {
        "row" => Some(GridAutoFlow::Row),
        "column" => Some(GridAutoFlow::Column),
        "dense" => Some(GridAutoFlow::RowDense),
        _ => None,
    }
}

fn parse_pair_flow_ident(first: &str, second: &str) -> Option<GridAutoFlow> {
    let a = first.to_ascii_lowercase();
    let b = second.to_ascii_lowercase();
    match (a.as_str(), b.as_str()) {
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
        [Token::Number(num)] => parse_signed_line_number(*num).map(GridPlacement::Line),
        [Token::Ident(span_kw), Token::Number(num)] if span_kw.eq_ignore_ascii_case("span") => {
            parse_positive_span(*num).map(GridPlacement::Span)
        }
        [Token::Ident(span_kw), Token::Ident(name)] if span_kw.eq_ignore_ascii_case("span") => {
            GridLineName::new(name.as_str()).map(|n| GridPlacement::SpanNamed(GridSpan::ONE, n))
        }
        [
            Token::Ident(span_kw),
            Token::Number(num),
            Token::Ident(name),
        ] if span_kw.eq_ignore_ascii_case("span") => {
            let span = parse_positive_span(*num)?;
            let line_name = GridLineName::new(name.as_str())?;
            Some(GridPlacement::SpanNamed(span, line_name))
        }
        [Token::Number(num), Token::Ident(name)] => {
            let line = parse_signed_line_number(*num)?;
            let line_name = GridLineName::new(name.as_str())?;
            Some(GridPlacement::LineNamed(line, line_name))
        }
        _ => None,
    }
}

fn parse_placement_ident(ident: &str) -> Option<GridPlacement> {
    if ident.eq_ignore_ascii_case("auto") {
        return Some(GridPlacement::Auto);
    }
    GridLineName::new(ident).map(GridPlacement::Named)
}

fn parse_signed_line_number(num: f32) -> Option<GridLine> {
    format!("{num}").parse::<i32>().ok().and_then(GridLine::new)
}

fn parse_positive_span(num: f32) -> Option<GridSpan> {
    format!("{num}").parse::<u32>().ok().and_then(GridSpan::new)
}

/// Parses placement shorthand for one axis (`grid-column: start / end`).
#[must_use]
pub fn parse_placement_shorthand(tokens: &[Token]) -> Option<(GridPlacement, GridPlacement)> {
    let parts: Vec<&[Token]> = tokens
        .split(|t| matches!(t, Token::Delimiter('/')))
        .collect();
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
#[must_use]
pub fn parse_grid_area_shorthand(
    tokens: &[Token],
) -> Option<(GridPlacement, GridPlacement, GridPlacement, GridPlacement)> {
    let parts: Vec<&[Token]> = tokens
        .split(|t| matches!(t, Token::Delimiter('/')))
        .collect();
    match parts.as_slice() {
        [single] => expand_single_area(single),
        [r_s, c_s] => {
            let row_start = parse_grid_line_placement(r_s)?;
            let col_start = parse_grid_line_placement(c_s)?;
            Some((
                row_start,
                col_start,
                GridPlacement::Auto,
                GridPlacement::Auto,
            ))
        }
        [r_s, c_s, r_e] => {
            let row_start = parse_grid_line_placement(r_s)?;
            let col_start = parse_grid_line_placement(c_s)?;
            let row_end = parse_grid_line_placement(r_e)?;
            Some((row_start, col_start, row_end, GridPlacement::Auto))
        }
        [r_s, c_s, r_e, c_e] => {
            let row_start = parse_grid_line_placement(r_s)?;
            let col_start = parse_grid_line_placement(c_s)?;
            let row_end = parse_grid_line_placement(r_e)?;
            let col_end = parse_grid_line_placement(c_e)?;
            Some((row_start, col_start, row_end, col_end))
        }
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
