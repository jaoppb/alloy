//! Box shadow parser (CSS Backgrounds & Borders L3 §7.1).

use crate::domain::color::CssColor;
use crate::domain::computed::visual::{BoxShadow, BoxShadowList, ShadowPlacement};
use crate::domain::length::Length;
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::values::{
    component_values, parse_color, parse_length, split_top_level_commas,
};

/// The most lengths one shadow takes: offset-x, offset-y, blur, spread.
const MAX_SHADOW_LENGTHS: usize = 4;

/// `box-shadow`: `none`, or a comma-separated list of shadows.
#[must_use]
pub fn parse_box_shadow(tokens: &[Token]) -> Option<BoxShadowList> {
    match tokens {
        [Token::Ident(name)] if name.eq_ignore_ascii_case("none") => Some(BoxShadowList::none()),
        _ => parse_shadow_list(tokens),
    }
}

/// The list splits at top-level commas only, so the commas inside an
/// `rgba(…)` colour stay with their shadow. More than
/// [`BoxShadowList::CAPACITY`] shadows is refused with a warning, never
/// truncated into a different paint.
fn parse_shadow_list(tokens: &[Token]) -> Option<BoxShadowList> {
    let layers = split_top_level_commas(tokens);
    if layers.len() > BoxShadowList::CAPACITY {
        tracing::warn!(
            shadows = layers.len(),
            capacity = BoxShadowList::CAPACITY,
            "box-shadow exceeds the fixed shadow capacity; declaration rejected"
        );
        return None;
    }
    let mut entries = [None; BoxShadowList::CAPACITY];
    for (slot, layer) in entries.iter_mut().zip(&layers) {
        *slot = Some(parse_single_shadow(layer)?);
    }
    Some(BoxShadowList::from_entries(entries))
}

/// `<shadow> = <color>? && <length>{2,4} && inset?`. Every component must be
/// one of the three; a repeated colour or `inset`, a fifth length or any other
/// token refuses the whole declaration.
fn parse_single_shadow(tokens: &[Token]) -> Option<BoxShadow> {
    let parts = component_values(tokens).try_fold(ShadowParts::default(), ShadowParts::absorb)?;
    parts.build()
}

/// The components of one shadow, as they are read.
#[derive(Default)]
struct ShadowParts {
    lengths: Vec<Length>,
    color: Option<CssColor>,
    placement: Option<ShadowPlacement>,
}

impl ShadowParts {
    fn absorb(self, component: &[Token]) -> Option<Self> {
        if is_inset(component) {
            return self.with_inset();
        }
        if let Some(length) = parse_length(component) {
            return self.with_length(length);
        }
        let color = parse_color(component)?;
        self.with_color(color)
    }

    fn with_inset(self) -> Option<Self> {
        if self.placement.is_some() {
            return None;
        }
        Some(Self {
            placement: Some(ShadowPlacement::Inset),
            ..self
        })
    }

    fn with_length(mut self, length: Length) -> Option<Self> {
        if self.lengths.len() >= MAX_SHADOW_LENGTHS {
            return None;
        }
        self.lengths.push(length);
        Some(self)
    }

    fn with_color(self, color: CssColor) -> Option<Self> {
        if self.color.is_some() {
            return None;
        }
        Some(Self {
            color: Some(color),
            ..self
        })
    }

    /// The shadow, with CSS's defaults for what was omitted: black (standing
    /// in for `currentcolor`, which this cut does not resolve), zero blur and
    /// spread, outset.
    fn build(self) -> Option<BoxShadow> {
        let color = self.color.unwrap_or(CssColor::BLACK);
        let placement = self.placement.unwrap_or(ShadowPlacement::Outset);
        let (offset_x, offset_y, blur, spread) = match self.lengths.as_slice() {
            [offset_x, offset_y] => (*offset_x, *offset_y, Length::ZERO, Length::ZERO),
            [offset_x, offset_y, blur] => (*offset_x, *offset_y, *blur, Length::ZERO),
            [offset_x, offset_y, blur, spread] => (*offset_x, *offset_y, *blur, *spread),
            _ => return None,
        };
        Some(BoxShadow::new(
            offset_x, offset_y, blur, spread, color, placement,
        ))
    }
}

fn is_inset(component: &[Token]) -> bool {
    matches!(component, [Token::Ident(name)] if name.eq_ignore_ascii_case("inset"))
}
