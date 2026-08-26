use crate::domain::computed::ComputedStyle;
use dom::NodeId;

/// A DOM node combined with its resolved `ComputedStyle` and styled children.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledNode {
    node_id: NodeId,
    style: ComputedStyle,
    children: Vec<StyledNode>,
}

impl StyledNode {
    /// Creates a new styled node.
    #[must_use]
    pub const fn new(node_id: NodeId, style: ComputedStyle, children: Vec<StyledNode>) -> Self {
        Self {
            node_id,
            style,
            children,
        }
    }

    /// Accesses the underlying DOM NodeId.
    #[must_use]
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Accesses the resolved computed style.
    #[must_use]
    pub const fn style(&self) -> &ComputedStyle {
        &self.style
    }

    /// Accesses the styled children.
    #[must_use]
    pub fn children(&self) -> &[StyledNode] {
        &self.children
    }
}

/// Aggregate root representing the styled DOM hierarchy (ADR-0010, CLAUDE.md).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyledTree {
    root: Option<StyledNode>,
}

impl StyledTree {
    /// Constructs a new `StyledTree`.
    #[must_use]
    pub const fn new(root: Option<StyledNode>) -> Self {
        Self { root }
    }

    /// Accesses the root styled node.
    #[must_use]
    pub const fn root(&self) -> Option<&StyledNode> {
        self.root.as_ref()
    }

    /// Checks if the styled tree is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.root.is_none()
    }
}
