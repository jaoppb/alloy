//! [`TextAdvanceStyle`] and the typography / line formatting types.

pub mod decoration;
pub mod font_style;
pub mod font_weight;
pub mod line_height;
pub mod overflow;
pub mod spacing;
pub mod style;
pub mod transform;
pub mod wrap;

pub use decoration::{TextDecoration, TextDecorationLine, TextDecorationStyle};
pub use font_style::FontStyle;
pub use font_weight::FontWeight;
pub use line_height::{LineHeight, LineHeightFactor, LineHeightPercentage};
pub use overflow::TextOverflow;
pub use spacing::{LetterSpacing, WordSpacing};
pub use style::TextAdvanceStyle;
pub use transform::TextTransform;
pub use wrap::{OverflowWrap, WordBreak};
