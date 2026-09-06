//! Physical and logical sides of a box (CSS Logical Properties L1 §2).

/// A physical side of a box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PhysicalSide {
    Top,
    Right,
    Bottom,
    Left,
}

/// A flow-relative logical side of a box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LogicalSide {
    BlockStart,
    BlockEnd,
    InlineStart,
    InlineEnd,
}
