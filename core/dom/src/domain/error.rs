use crate::domain::node_id::NodeId;
use std::fmt;

/// Domain errors representing failures in DOM tree manipulation or validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomError {
    /// Node was not found in the DOM arena.
    NodeNotFound(NodeId),
    /// Appending or inserting would cause a cycle in the tree hierarchy.
    CycleDetected {
        /// The node being attached.
        node: NodeId,
        /// The intended parent where a cycle is detected.
        parent: NodeId,
    },
    /// Invalid tree hierarchy operation (e.g. attempting to make document child of an element).
    InvalidHierarchy(String),
    /// Invalid tag name provided for an element node.
    InvalidTagName(String),
}

impl fmt::Display for DomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeNotFound(id) => write!(f, "Node not found in DOM arena: {id}"),
            Self::CycleDetected { node, parent } => {
                write!(
                    f,
                    "Cycle detected: cannot attach node {node} under descendant {parent}"
                )
            }
            Self::InvalidHierarchy(msg) => write!(f, "Invalid DOM hierarchy: {msg}"),
            Self::InvalidTagName(tag) => write!(f, "Invalid DOM element tag name: '{tag}'"),
        }
    }
}

impl std::error::Error for DomError {}
