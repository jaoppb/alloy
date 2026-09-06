//! Computed-value types: what a property becomes after the cascade
//! (`PRD-007:39`).

pub mod display;
pub mod edges;
pub mod flex;
pub mod font;
pub mod grid;
pub mod inline_style;
pub mod intrinsic;
pub mod logical;
pub mod overflow;
pub mod position;
pub mod sizing;
pub mod sizing_constraints;
pub mod style;
pub mod text_advance;
pub mod variables;
pub mod visual;

pub use display::Display;
pub use edges::LengthEdges;
pub use flex::{
    AlignContent, AlignItems, AlignSelf, FlexDirection, FlexFactor, FlexStyle, FlexWrap,
    JustifyContent,
};
pub use font::{FamilyName, FontFamily, FontFamilyList, GenericFamily};
pub use grid::{
    GridAutoFlow, GridFr, GridGap, GridLine, GridLineName, GridPlacement, GridSpan, GridStyle,
    GridTemplateAreas, MaxTrackBreadth, MinTrackBreadth, TrackList, TrackSize,
};
pub use inline_style::{TextAlign, WhiteSpace};
pub use intrinsic::IntrinsicSize;
pub use logical::{
    Direction, LogicalEdges, LogicalInsets, LogicalSizing, LogicalStyle, WritingContext,
    WritingMode,
};
pub use overflow::{Overflow, OverflowStyle};
pub use position::{PositionStyle, PositionType, ZIndex};
pub use sizing::{BoxSizing, Sizing};
pub use sizing_constraints::SizingConstraints;
pub use style::ComputedStyle;
pub use text_advance::{
    FontStyle, FontWeight, LetterSpacing, LineHeight, LineHeightFactor, LineHeightPercentage,
    OverflowWrap, TextAdvanceStyle, TextDecoration, TextDecorationLine, TextDecorationStyle,
    TextOverflow, TextTransform, WordBreak, WordSpacing,
};
pub use variables::{CustomPropertiesMap, VariableName, VariableValue};
pub use visual::{
    BackgroundImage, BackgroundPosition, BackgroundRepeat, BackgroundSize, BorderColorEdges,
    BorderRadius, BorderStyle, BorderStyleEdges, BoxShadow, BoxShadowList, Opacity,
    ShadowPlacement, VisualStyle,
};
