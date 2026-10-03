//! Physical and logical axes of a box (CSS Logical Properties L1 §2).

/// A physical axis of a box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PhysicalAxis {
    Horizontal,
    Vertical,
}

/// A flow-relative logical axis of a box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LogicalAxis {
    Block,
    Inline,
}
