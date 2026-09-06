//! Box shadow parser.

use crate::domain::color::CssColor;
use crate::domain::computed::visual::{BoxShadow, BoxShadowList, ShadowPlacement};
use crate::domain::length::Length;
use crate::infrastructure::parser::token::Token;

use super::helpers::{length_from_token, parse_single_color};

#[must_use]
pub fn parse_box_shadow(tokens: &[Token]) -> Option<BoxShadowList> {
    match tokens {
        [Token::Ident(name)] if name.eq_ignore_ascii_case("none") => Some(BoxShadowList::none()),
        _ => parse_shadow_list(tokens),
    }
}

fn parse_shadow_list(tokens: &[Token]) -> Option<BoxShadowList> {
    let mut entries = [None; BoxShadowList::CAPACITY];
    let mut index: usize = 0;
    for part in tokens.split(|token| matches!(token, Token::Comma)) {
        let shadow = parse_single_shadow(part)?;
        if let Some(slot) = entries.get_mut(index) {
            *slot = Some(shadow);
            index = index.saturating_add(1);
        }
    }
    match index {
        0 => None,
        _ => Some(BoxShadowList::from_entries(entries)),
    }
}

fn parse_single_shadow(tokens: &[Token]) -> Option<BoxShadow> {
    let mut placement = ShadowPlacement::Outset;
    let mut color = None;
    let mut lengths: [Option<Length>; 4] = [None; 4];
    let mut length_count: usize = 0;

    for token in tokens {
        match token {
            Token::Ident(name) if name.eq_ignore_ascii_case("inset") => {
                placement = ShadowPlacement::Inset;
            }
            _ => {
                if let Some(length) = length_from_token(token) {
                    if let Some(slot) = lengths.get_mut(length_count) {
                        *slot = Some(length);
                        length_count = length_count.saturating_add(1);
                    }
                    continue;
                }
                if let Some(c) = parse_single_color(core::slice::from_ref(token)) {
                    color = Some(c);
                }
            }
        }
    }

    let resolved_color = color.unwrap_or(CssColor::BLACK);
    build_shadow(lengths, length_count, resolved_color, placement)
}

fn build_shadow(
    lengths: [Option<Length>; 4],
    count: usize,
    color: CssColor,
    placement: ShadowPlacement,
) -> Option<BoxShadow> {
    match count {
        2 => {
            let h = lengths.first()?.unwrap_or(Length::ZERO);
            let v = lengths.get(1)?.unwrap_or(Length::ZERO);
            Some(BoxShadow::new(
                h,
                v,
                Length::ZERO,
                Length::ZERO,
                color,
                placement,
            ))
        }
        3 => {
            let h = lengths.first()?.unwrap_or(Length::ZERO);
            let v = lengths.get(1)?.unwrap_or(Length::ZERO);
            let blur = lengths.get(2)?.unwrap_or(Length::ZERO);
            Some(BoxShadow::new(h, v, blur, Length::ZERO, color, placement))
        }
        4 => {
            let h = lengths.first()?.unwrap_or(Length::ZERO);
            let v = lengths.get(1)?.unwrap_or(Length::ZERO);
            let blur = lengths.get(2)?.unwrap_or(Length::ZERO);
            let spread = lengths.get(3)?.unwrap_or(Length::ZERO);
            Some(BoxShadow::new(h, v, blur, spread, color, placement))
        }
        _ => None,
    }
}
