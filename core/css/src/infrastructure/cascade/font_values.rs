//! The `font-family` half of the cascade (v0.5 fonts increment): the one
//! list-valued property, applied, reset to `initial` and copied for `inherit`.
//!
//! Split out of `values.rs` for the reason `flex_values.rs` gives — one more
//! property with its own value grammar and its own `apply` / `reset` / `inherit`
//! triple is a page `values.rs` does not need to carry.

use crate::domain::computed::style::ComputedStyle;
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::values::parse_font_family;

/// The property this module owns.
const FONT_FAMILY: &str = "font-family";

/// `style` with `font-family` set from `tokens`, or `None` when `property` is
/// not `font-family` or the value has no readable entry.
pub(crate) fn apply(
    style: ComputedStyle,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    if property != FONT_FAMILY {
        return None;
    }
    parse_font_family(tokens).map(|list| style.with_font_family(list))
}

/// `style` with `font-family` reset to its CSS `initial` value (an empty list).
pub(crate) fn reset(style: ComputedStyle, property: &str) -> Option<ComputedStyle> {
    if property != FONT_FAMILY {
        return None;
    }
    Some(style.with_font_family(ComputedStyle::initial().font_family()))
}

/// `style` with `font-family` copied from `parent` — the `inherit` keyword.
/// `font-family` inherits on its own, so this only matters when an author
/// writes the keyword explicitly on a node whose parent set a non-initial list.
pub(crate) fn inherit(
    style: ComputedStyle,
    parent: &ComputedStyle,
    property: &str,
) -> Option<ComputedStyle> {
    if property != FONT_FAMILY {
        return None;
    }
    Some(style.with_font_family(parent.font_family()))
}
