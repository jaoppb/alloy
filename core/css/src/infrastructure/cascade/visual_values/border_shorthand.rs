//! Border shorthand parsers.

use crate::domain::computed::visual::{BorderColorEdges, BorderRadius, BorderStyleEdges};
use crate::infrastructure::parser::token::Token;

use super::border::{parse_border_color, parse_border_radius_corner, parse_border_style};

#[must_use]
pub fn parse_border_style_shorthand(tokens: &[Token]) -> Option<BorderStyleEdges> {
    match tokens {
        [s1] => parse_border_style(core::slice::from_ref(s1)).map(BorderStyleEdges::uniform),
        [s1, s2] => {
            let top_bottom = parse_border_style(core::slice::from_ref(s1))?;
            let right_left = parse_border_style(core::slice::from_ref(s2))?;
            Some(BorderStyleEdges::new(
                top_bottom, right_left, top_bottom, right_left,
            ))
        }
        [s1, s2, s3] => {
            let top = parse_border_style(core::slice::from_ref(s1))?;
            let right_left = parse_border_style(core::slice::from_ref(s2))?;
            let bottom = parse_border_style(core::slice::from_ref(s3))?;
            Some(BorderStyleEdges::new(top, right_left, bottom, right_left))
        }
        [s1, s2, s3, s4] => {
            let top = parse_border_style(core::slice::from_ref(s1))?;
            let right = parse_border_style(core::slice::from_ref(s2))?;
            let bottom = parse_border_style(core::slice::from_ref(s3))?;
            let left = parse_border_style(core::slice::from_ref(s4))?;
            Some(BorderStyleEdges::new(top, right, bottom, left))
        }
        _ => None,
    }
}

#[must_use]
pub fn parse_border_color_shorthand(tokens: &[Token]) -> Option<BorderColorEdges> {
    match tokens {
        [c1] => parse_border_color(core::slice::from_ref(c1)).map(BorderColorEdges::uniform),
        [c1, c2] => {
            let top_bottom = parse_border_color(core::slice::from_ref(c1))?;
            let right_left = parse_border_color(core::slice::from_ref(c2))?;
            Some(BorderColorEdges::new(
                top_bottom, right_left, top_bottom, right_left,
            ))
        }
        [c1, c2, c3] => {
            let top = parse_border_color(core::slice::from_ref(c1))?;
            let right_left = parse_border_color(core::slice::from_ref(c2))?;
            let bottom = parse_border_color(core::slice::from_ref(c3))?;
            Some(BorderColorEdges::new(top, right_left, bottom, right_left))
        }
        [c1, c2, c3, c4] => {
            let top = parse_border_color(core::slice::from_ref(c1))?;
            let right = parse_border_color(core::slice::from_ref(c2))?;
            let bottom = parse_border_color(core::slice::from_ref(c3))?;
            let left = parse_border_color(core::slice::from_ref(c4))?;
            Some(BorderColorEdges::new(top, right, bottom, left))
        }
        _ => None,
    }
}

#[must_use]
pub fn parse_border_radius_shorthand(tokens: &[Token]) -> Option<BorderRadius> {
    match tokens {
        [r1] => parse_border_radius_corner(core::slice::from_ref(r1)).map(BorderRadius::uniform),
        [r1, r2] => {
            let tl_br = parse_border_radius_corner(core::slice::from_ref(r1))?;
            let tr_bl = parse_border_radius_corner(core::slice::from_ref(r2))?;
            Some(BorderRadius::new(tl_br, tr_bl, tl_br, tr_bl))
        }
        [r1, r2, r3] => {
            let tl = parse_border_radius_corner(core::slice::from_ref(r1))?;
            let tr_bl = parse_border_radius_corner(core::slice::from_ref(r2))?;
            let br = parse_border_radius_corner(core::slice::from_ref(r3))?;
            Some(BorderRadius::new(tl, tr_bl, br, tr_bl))
        }
        [r1, r2, r3, r4] => {
            let tl = parse_border_radius_corner(core::slice::from_ref(r1))?;
            let tr = parse_border_radius_corner(core::slice::from_ref(r2))?;
            let br = parse_border_radius_corner(core::slice::from_ref(r3))?;
            let bl = parse_border_radius_corner(core::slice::from_ref(r4))?;
            Some(BorderRadius::new(tl, tr, br, bl))
        }
        _ => None,
    }
}
