//! CSS Logical Properties and Values Level 1 domain models (CSS Logical Properties L1).
//!
//! Provides flow-relative types ([`WritingMode`], [`Direction`], [`WritingContext`],
//! [`LogicalEdges`], [`LogicalSizing`], [`LogicalInsets`], [`LogicalStyle`]) and their
//! mapping to physical box geometry.

#[path = "logical/axis.rs"]
pub mod axis;
#[path = "logical/direction.rs"]
pub mod direction;
#[path = "logical/edges.rs"]
pub mod edges;
#[path = "logical/insets.rs"]
pub mod insets;
#[path = "logical/mapping.rs"]
pub mod mapping;
#[path = "logical/side.rs"]
pub mod side;
#[path = "logical/sizing.rs"]
pub mod sizing;
#[path = "logical/style.rs"]
pub mod style;
#[path = "logical/writing_mode.rs"]
pub mod writing_mode;

pub use axis::{LogicalAxis, PhysicalAxis};
pub use direction::Direction;
pub use edges::LogicalEdges;
pub use insets::LogicalInsets;
pub use mapping::WritingContext;
pub use side::{LogicalSide, PhysicalSide};
pub use sizing::LogicalSizing;
pub use style::LogicalStyle;
pub use writing_mode::WritingMode;
