//! [`apply_declaration`] — one parsed [`Declaration`] folded into a
//! [`ComputedStyle`].
//!
//! The registry `crate::SUPPORTED_PROPERTIES` names what may arrive here and
//! `tests/data/MANIFEST.md` names the same set, checked both ways by
//! `tests/manifest_runner.rs`. A declaration whose **value** is outside the cut
//! — `rgb()`, a `vh` length, a colour name B2 has not brought in yet — answers
//! `None`, and the caller leaves the previous value standing. Painting an
//! arbitrary colour instead would be the silent shrinkage `relatório
//! §2.8:350-354` forbids.

use crate::domain::computed::edges::LengthEdges;
use crate::domain::computed::style::ComputedStyle;
use crate::domain::declaration::Declaration;
use crate::domain::length::Length;
use crate::infrastructure::parser::values::{
    parse_color, parse_display, parse_length, parse_length_edges, value_tokens,
};

/// Which box property a `-top` / `-right` / `-bottom` / `-left` longhand edits.
#[derive(Clone, Copy)]
enum BoxProperty {
    Margin,
    Padding,
}

/// Which of the four sides that longhand names.
#[derive(Clone, Copy)]
enum BoxSide {
    Top,
    Right,
    Bottom,
    Left,
}

/// `style` with `declaration` applied, or `None` when the value is outside the
/// v0.5 cut.
#[must_use]
pub(crate) fn apply_declaration(
    style: ComputedStyle,
    declaration: &Declaration,
) -> Option<ComputedStyle> {
    let tokens = value_tokens(declaration.value());
    apply_property(style, declaration.property().as_str(), &tokens)
}

/// The six shorthand and singular properties; the eight edge longhands fall
/// through to [`apply_edge_longhand`].
fn apply_property(
    style: ComputedStyle,
    property: &str,
    tokens: &[crate::infrastructure::parser::Token],
) -> Option<ComputedStyle> {
    match property {
        "display" => parse_display(tokens).map(|value| style.with_display(value)),
        "color" => parse_color(tokens).map(|value| style.with_color(value)),
        "background-color" => parse_color(tokens).map(|value| style.with_background_color(value)),
        "font-size" => parse_length(tokens).map(|value| style.with_font_size(value)),
        "margin" => parse_length_edges(tokens).map(|value| style.with_margin(value)),
        "padding" => parse_length_edges(tokens).map(|value| style.with_padding(value)),
        _ => apply_edge_longhand(style, property, tokens),
    }
}

fn apply_edge_longhand(
    style: ComputedStyle,
    property: &str,
    tokens: &[crate::infrastructure::parser::Token],
) -> Option<ComputedStyle> {
    let (box_property, side) = split_edge_property(property)?;
    let length = parse_length(tokens)?;
    Some(set_edge(style, box_property, side, length))
}

fn split_edge_property(property: &str) -> Option<(BoxProperty, BoxSide)> {
    let margin = property
        .strip_prefix("margin-")
        .map(|side| (BoxProperty::Margin, side));
    let padding = property
        .strip_prefix("padding-")
        .map(|side| (BoxProperty::Padding, side));
    let (box_property, name) = margin.or(padding)?;
    Some((box_property, box_side(name)?))
}

fn box_side(name: &str) -> Option<BoxSide> {
    match name {
        "top" => Some(BoxSide::Top),
        "right" => Some(BoxSide::Right),
        "bottom" => Some(BoxSide::Bottom),
        "left" => Some(BoxSide::Left),
        _ => None,
    }
}

const fn set_edge(
    style: ComputedStyle,
    box_property: BoxProperty,
    side: BoxSide,
    length: Length,
) -> ComputedStyle {
    match box_property {
        BoxProperty::Margin => style.with_margin(edge_with(style.margin(), side, length)),
        BoxProperty::Padding => style.with_padding(edge_with(style.padding(), side, length)),
    }
}

const fn edge_with(edges: LengthEdges, side: BoxSide, length: Length) -> LengthEdges {
    match side {
        BoxSide::Top => edges.with_top(length),
        BoxSide::Right => edges.with_right(length),
        BoxSide::Bottom => edges.with_bottom(length),
        BoxSide::Left => edges.with_left(length),
    }
}
