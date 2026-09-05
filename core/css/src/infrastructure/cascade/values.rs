//! [`apply_declaration`] — one parsed [`Declaration`] folded into a
//! [`ComputedStyle`].
//!
//! The registry `crate::SUPPORTED_PROPERTIES` names what may arrive here and
//! `tests/data/MANIFEST.md` names the same set, checked both ways by
//! `tests/manifest_runner.rs`. A declaration whose **value** is outside the
//! cut — a `vh` length, say — answers `None`, and the caller leaves the
//! previous value standing. Painting an arbitrary colour instead would be the
//! silent shrinkage `relatório §2.8:350-354` forbids.
//!
//! B2 (`plano:435-443`) adds the two CSS-wide keywords of CSS Cascade L4
//! §7.1: `initial` resets *any* listed property to
//! [`ComputedStyle::initial`]'s value for it, and `inherit` forces
//! inheritance even for a property — `display`, `background-color`, every
//! box edge — that does not normally inherit. Both are checked ahead of a
//! property's own value grammar, because they are not part of it.

use crate::domain::computed::edges::LengthEdges;
use crate::domain::computed::style::ComputedStyle;
use crate::domain::declaration::Declaration;
use crate::domain::length::Length;
use crate::infrastructure::cascade::flex_values;
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::values::{
    parse_box_sizing, parse_color, parse_display, parse_length, parse_length_edges, parse_sizing,
    parse_text_align, parse_white_space, value_tokens,
};

/// Which box property a `-top` / `-right` / `-bottom` / `-left` longhand edits.
#[derive(Clone, Copy)]
enum BoxProperty {
    Margin,
    Border,
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

/// `initial` or `inherit` (CSS Cascade L4 §7.1) — the two CSS-wide keywords
/// this cut recognises. `unset` and `revert` are not: neither has a reading
/// that does not depend on a property's own inherited-ness table, which this
/// crate does not carry yet.
#[derive(Clone, Copy)]
enum CssWideKeyword {
    Initial,
    Inherit,
}

/// `style` with `declaration` applied, or `None` when the value is outside the
/// v0.5 cut.
#[must_use]
pub(crate) fn apply_declaration(
    style: ComputedStyle,
    declaration: &Declaration,
    parent: Option<&ComputedStyle>,
) -> Option<ComputedStyle> {
    let tokens = value_tokens(declaration.value());
    apply_property(style, parent, declaration.property().as_str(), &tokens)
}

fn apply_property(
    style: ComputedStyle,
    parent: Option<&ComputedStyle>,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    css_wide_keyword(tokens)
        .and_then(|keyword| apply_css_wide_keyword(style, parent, property, keyword))
        .or_else(|| apply_property_value(style, property, tokens))
}

/// The shorthand and singular properties; the twelve edge longhands fall
/// through to [`apply_edge_longhand`] and the nine Flexbox ones to
/// [`flex_values::apply`].
fn apply_property_value(
    style: ComputedStyle,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    match property {
        "display" => parse_display(tokens).map(|value| style.with_display(value)),
        "color" => parse_color(tokens).map(|value| style.with_color(value)),
        "background-color" => parse_color(tokens).map(|value| style.with_background_color(value)),
        "font-size" => parse_length(tokens).map(|value| style.with_font_size(value)),
        "margin" => parse_length_edges(tokens).map(|value| style.with_margin(value)),
        "border-width" => parse_length_edges(tokens).map(|value| style.with_border(value)),
        "padding" => parse_length_edges(tokens).map(|value| style.with_padding(value)),
        "width" => parse_sizing(tokens).map(|value| style.with_width(value)),
        "height" => parse_sizing(tokens).map(|value| style.with_height(value)),
        "box-sizing" => parse_box_sizing(tokens).map(|value| style.with_box_sizing(value)),
        "text-align" => parse_text_align(tokens).map(|value| style.with_text_align(value)),
        "white-space" => parse_white_space(tokens).map(|value| style.with_white_space(value)),
        _ => apply_edge_or_flex(style, property, tokens),
    }
}

fn apply_edge_or_flex(
    style: ComputedStyle,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    apply_edge_longhand(style, property, tokens)
        .or_else(|| flex_values::apply(style, property, tokens))
}

fn apply_edge_longhand(
    style: ComputedStyle,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    let (box_property, side) = split_edge_property(property)?;
    let length = parse_length(tokens)?;
    Some(set_edge(style, box_property, side, length))
}

/// `margin-top`, `padding-left`, `border-right-width`, … → which box property
/// and which side. The `border` longhands carry a `-width` suffix because only
/// the **width** of a border is geometry; `border-style` and `border-color` are
/// paint and stay outside the cut.
fn split_edge_property(property: &str) -> Option<(BoxProperty, BoxSide)> {
    let margin = property
        .strip_prefix("margin-")
        .map(|side| (BoxProperty::Margin, side));
    let padding = property
        .strip_prefix("padding-")
        .map(|side| (BoxProperty::Padding, side));
    let border = border_side_name(property).map(|side| (BoxProperty::Border, side));
    let (box_property, name) = margin.or(padding).or(border)?;
    Some((box_property, box_side(name)?))
}

fn border_side_name(property: &str) -> Option<&str> {
    property
        .strip_prefix("border-")
        .and_then(|rest| rest.strip_suffix("-width"))
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
        BoxProperty::Border => style.with_border(edge_with(style.border(), side, length)),
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

const fn edge_value(edges: LengthEdges, side: BoxSide) -> Length {
    match side {
        BoxSide::Top => edges.top(),
        BoxSide::Right => edges.right(),
        BoxSide::Bottom => edges.bottom(),
        BoxSide::Left => edges.left(),
    }
}

// ---- CSS-wide keywords (CSS Cascade L4 §7.1) ------------------------------

fn css_wide_keyword(tokens: &[Token]) -> Option<CssWideKeyword> {
    let [Token::Ident(name)] = tokens else {
        return None;
    };
    match name.to_ascii_lowercase().as_str() {
        "initial" => Some(CssWideKeyword::Initial),
        "inherit" => Some(CssWideKeyword::Inherit),
        _ => None,
    }
}

fn apply_css_wide_keyword(
    style: ComputedStyle,
    parent: Option<&ComputedStyle>,
    property: &str,
    keyword: CssWideKeyword,
) -> Option<ComputedStyle> {
    match keyword {
        CssWideKeyword::Initial => reset_to_initial(style, property),
        CssWideKeyword::Inherit => inherit_property(style, parent, property),
    }
}

/// `style` with `property` reset to the value [`ComputedStyle::initial`]
/// gives it, ignoring whatever the parent computed.
fn reset_to_initial(style: ComputedStyle, property: &str) -> Option<ComputedStyle> {
    let initial = ComputedStyle::initial();
    match property {
        "display" => Some(style.with_display(initial.display())),
        "color" => Some(style.with_color(initial.color())),
        "background-color" => Some(style.with_background_color(initial.background_color())),
        "margin" => Some(style.with_margin(initial.margin())),
        "border-width" => Some(style.with_border(initial.border())),
        "padding" => Some(style.with_padding(initial.padding())),
        "font-size" => Some(style.with_font_size(initial.font_size())),
        "width" => Some(style.with_width(initial.width())),
        "height" => Some(style.with_height(initial.height())),
        "box-sizing" => Some(style.with_box_sizing(initial.box_sizing())),
        "text-align" => Some(style.with_text_align(initial.text_align())),
        "white-space" => Some(style.with_white_space(initial.white_space())),
        _ => reset_edge_or_flex(style, property),
    }
}

fn reset_edge_or_flex(style: ComputedStyle, property: &str) -> Option<ComputedStyle> {
    reset_edge_longhand(style, property).or_else(|| flex_values::reset(style, property))
}

fn reset_edge_longhand(style: ComputedStyle, property: &str) -> Option<ComputedStyle> {
    let (box_property, side) = split_edge_property(property)?;
    Some(set_edge(style, box_property, side, Length::ZERO))
}

/// `style` with `property` copied from `parent`, forcing inheritance even for
/// a property that does not normally inherit. A node with no parent (the
/// document root) has nothing to inherit from, so `inherit` there computes to
/// `initial` — CSS Cascade L4 §7.1, "on the root element, `inherit`...
/// computes to the property's initial value".
fn inherit_property(
    style: ComputedStyle,
    parent: Option<&ComputedStyle>,
    property: &str,
) -> Option<ComputedStyle> {
    parent.map_or_else(
        || reset_to_initial(style, property),
        |parent| copy_property(style, parent, property),
    )
}

fn copy_property(
    style: ComputedStyle,
    parent: &ComputedStyle,
    property: &str,
) -> Option<ComputedStyle> {
    match property {
        "display" => Some(style.with_display(parent.display())),
        "color" => Some(style.with_color(parent.color())),
        "background-color" => Some(style.with_background_color(parent.background_color())),
        "margin" => Some(style.with_margin(parent.margin())),
        "border-width" => Some(style.with_border(parent.border())),
        "padding" => Some(style.with_padding(parent.padding())),
        "font-size" => Some(style.with_font_size(parent.font_size())),
        "width" => Some(style.with_width(parent.width())),
        "height" => Some(style.with_height(parent.height())),
        "box-sizing" => Some(style.with_box_sizing(parent.box_sizing())),
        "text-align" => Some(style.with_text_align(parent.text_align())),
        "white-space" => Some(style.with_white_space(parent.white_space())),
        _ => copy_edge_or_flex(style, parent, property),
    }
}

fn copy_edge_or_flex(
    style: ComputedStyle,
    parent: &ComputedStyle,
    property: &str,
) -> Option<ComputedStyle> {
    copy_edge_longhand(style, parent, property)
        .or_else(|| flex_values::inherit(style, parent, property))
}

fn copy_edge_longhand(
    style: ComputedStyle,
    parent: &ComputedStyle,
    property: &str,
) -> Option<ComputedStyle> {
    let (box_property, side) = split_edge_property(property)?;
    let parent_edges = match box_property {
        BoxProperty::Margin => parent.margin(),
        BoxProperty::Border => parent.border(),
        BoxProperty::Padding => parent.padding(),
    };
    Some(set_edge(
        style,
        box_property,
        side,
        edge_value(parent_edges, side),
    ))
}
