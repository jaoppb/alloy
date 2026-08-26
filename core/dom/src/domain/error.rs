use crate::domain::node_id::NodeId;
use thiserror::Error;

/// Domain errors representing failures in DOM tree manipulation or validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomError {
    /// Node was not found in the DOM arena.
    #[error("Node not found in DOM arena: {0}")]
    NodeNotFound(NodeId),
    /// Appending or inserting would cause a cycle in the tree hierarchy.
    #[error("Cycle detected: cannot attach node {node} under descendant {parent}")]
    CycleDetected {
        /// The node being attached.
        node: NodeId,
        /// The intended parent where a cycle is detected.
        parent: NodeId,
    },
    /// Invalid tree hierarchy operation (e.g. attempting to make document child of an element).
    #[error("Invalid DOM hierarchy: {0}")]
    InvalidHierarchy(String),
    /// Invalid tag name provided for an element node.
    #[error("Invalid DOM element tag name: '{0}'")]
    InvalidTagName(String),
}
