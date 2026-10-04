//! Background position parser.

use crate::domain::computed::visual::BackgroundPosition;
use crate::domain::length::Length;
use crate::infrastructure::parser::token::Token;

use crate::infrastructure::parser::values::length_from_token;

/// The axis a `background-position` component can stand for (CSS
/// Backgrounds 3 §3.6): `left` / `right` are horizontal, `top` / `bottom`
/// vertical, and `center` or a length fits either.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PositionAxis {
    Horizontal,
    Vertical,
    Either,
}

impl PositionAxis {
    const fn fits_horizontal(self) -> bool {
        !matches!(self, Self::Vertical)
    }

    const fn fits_vertical(self) -> bool {
        !matches!(self, Self::Horizontal)
    }
}

/// One component of the value: the axis it names, its offset, and whether it
/// was written as a keyword — only a pair of keywords may come in either order.
#[derive(Clone, Copy)]
struct PositionComponent {
    axis: PositionAxis,
    offset: Length,
    is_keyword: bool,
}

impl PositionComponent {
    const fn keyword(axis: PositionAxis, percent: f32) -> Self {
        Self {
            axis,
            offset: Length::Percent(percent),
            is_keyword: true,
        }
    }

    fn of(token: &Token) -> Option<Self> {
        let Token::Ident(name) = token else {
            return Self::length(token);
        };
        match name.to_ascii_lowercase().as_str() {
            "left" => Some(Self::keyword(PositionAxis::Horizontal, 0.0)),
            "right" => Some(Self::keyword(PositionAxis::Horizontal, 100.0)),
            "top" => Some(Self::keyword(PositionAxis::Vertical, 0.0)),
            "bottom" => Some(Self::keyword(PositionAxis::Vertical, 100.0)),
            "center" => Some(Self::keyword(PositionAxis::Either, 50.0)),
            _ => None,
        }
    }

    fn length(token: &Token) -> Option<Self> {
        let offset = length_from_token(token)?;
        Some(Self {
            axis: PositionAxis::Either,
            offset,
            is_keyword: false,
        })
    }
}

#[must_use]
pub fn parse_background_position(tokens: &[Token]) -> Option<BackgroundPosition> {
    match tokens {
        [single] => Some(position_single(PositionComponent::of(single)?)),
        [first, second] => position_pair(
            PositionComponent::of(first)?,
            PositionComponent::of(second)?,
        ),
        _ => None,
    }
}

/// One component: it sets its own axis and the other is centred.
const fn position_single(component: PositionComponent) -> BackgroundPosition {
    match component.axis {
        PositionAxis::Vertical => BackgroundPosition::new(Length::Percent(50.0), component.offset),
        PositionAxis::Horizontal | PositionAxis::Either => {
            BackgroundPosition::new(component.offset, Length::Percent(50.0))
        }
    }
}

/// Two components: horizontal then vertical, except that two keywords may come
/// in either order (`top right` is `right top`). A pair naming the same axis
/// twice (`left right`) is invalid.
fn position_pair(
    first: PositionComponent,
    second: PositionComponent,
) -> Option<BackgroundPosition> {
    let (horizontal, vertical) = horizontal_first(first, second);
    if !horizontal.axis.fits_horizontal() || !vertical.axis.fits_vertical() {
        return None;
    }
    Some(BackgroundPosition::new(horizontal.offset, vertical.offset))
}

/// `first` and `second` reordered to horizontal-then-vertical when both are
/// keywords and written vertical-first.
fn horizontal_first(
    first: PositionComponent,
    second: PositionComponent,
) -> (PositionComponent, PositionComponent) {
    let both_keywords = first.is_keyword && second.is_keyword;
    let written_vertical_first =
        first.axis == PositionAxis::Vertical || second.axis == PositionAxis::Horizontal;
    if both_keywords && written_vertical_first {
        return (second, first);
    }
    (first, second)
}
