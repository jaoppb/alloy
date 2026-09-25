//! Hex color parsing utilities.

use crate::domain::color::CssColor;

#[must_use]
pub fn hex_color(digits: &str) -> Option<CssColor> {
    match digits.len() {
        3 => parse_hex_3(digits),
        6 => parse_hex_6(digits),
        _ => None,
    }
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte.wrapping_sub(b'0')),
        b'a'..=b'f' => Some(byte.wrapping_sub(b'a').saturating_add(10)),
        b'A'..=b'F' => Some(byte.wrapping_sub(b'A').saturating_add(10)),
        _ => None,
    }
}

fn parse_hex_3(digits: &str) -> Option<CssColor> {
    let bytes = digits.as_bytes();
    let r = hex_nibble(*bytes.first()?)?;
    let g = hex_nibble(*bytes.get(1)?)?;
    let b = hex_nibble(*bytes.get(2)?)?;
    let red = r.saturating_mul(16).saturating_add(r);
    let green = g.saturating_mul(16).saturating_add(g);
    let blue = b.saturating_mul(16).saturating_add(b);
    Some(CssColor::rgb(red, green, blue))
}

fn parse_hex_6(digits: &str) -> Option<CssColor> {
    let bytes = digits.as_bytes();
    let r1 = hex_nibble(*bytes.first()?)?;
    let r2 = hex_nibble(*bytes.get(1)?)?;
    let g1 = hex_nibble(*bytes.get(2)?)?;
    let g2 = hex_nibble(*bytes.get(3)?)?;
    let b1 = hex_nibble(*bytes.get(4)?)?;
    let b2 = hex_nibble(*bytes.get(5)?)?;
    let red = r1.saturating_mul(16).saturating_add(r2);
    let green = g1.saturating_mul(16).saturating_add(g2);
    let blue = b1.saturating_mul(16).saturating_add(b2);
    Some(CssColor::rgb(red, green, blue))
}
