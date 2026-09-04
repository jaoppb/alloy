//! Computed-value types: what a property becomes after the cascade
//! (`PRD-007:39`).

pub mod display;
pub mod edges;
pub mod style;

pub use display::Display;
pub use edges::LengthEdges;
pub use style::ComputedStyle;
