//! Computed-value types: what a property becomes after the cascade
//! (`PRD-007:39`).

pub mod display;
pub mod edges;
pub mod flex;
pub mod inline_style;
pub mod intrinsic;
pub mod sizing;
pub mod style;

pub use display::Display;
pub use edges::LengthEdges;
pub use flex::{
    AlignContent, AlignItems, AlignSelf, FlexDirection, FlexFactor, FlexStyle, FlexWrap,
    JustifyContent,
};
pub use inline_style::{TextAlign, WhiteSpace};
pub use intrinsic::IntrinsicSize;
pub use sizing::{BoxSizing, Sizing};
pub use style::ComputedStyle;
