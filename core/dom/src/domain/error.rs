//! [`DomError`] — the one typed error for `core/dom`.
//!
//! (`ADR-0011` item 4 applied to a domain crate). `core/dom` never names
//! `EngineError`; the `DomError` → `EngineError::Dom` mapping is the
//! `core/runtime/rhai` adapter's job at roadmap I1. `#[non_exhaustive]` so that
//! mapping keeps compiling as variants are added.

use crate::domain::node::NodeId;

#[derive(thiserror::Error, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DomError {
    /// The id addresses an empty (never-used or tombstoned) arena slot.
    #[error("{0} does not exist")]
    NodeNotFound(NodeId),
    /// The requested `append` / `insert` would make the tree cyclic.
    #[error("operation would make the tree cyclic")]
    WouldCycle,
    /// `parent` and `child` are the same node.
    #[error("a node cannot be its own parent")]
    SelfParent,
    /// `detach` / `remove` was called on the `Document` root.
    #[error("the document root cannot be detached or removed")]
    CannotDetachDocument,
    /// The chosen parent is a `Text` or `Comment` node and cannot hold children.
    #[error("{0} cannot hold children")]
    CannotHaveChildren(NodeId),
    /// A tag string failed [`crate::TagName`] validation.
    #[error("not a valid tag name: {0:?}")]
    InvalidTagName(String),
    /// An attribute name failed [`crate::AttributeName`] validation.
    #[error("not a valid attribute name: {0:?}")]
    InvalidAttributeName(String),
    /// A tag / attribute operation targeted a non-`Element` node.
    #[error("{0} is not an element")]
    NotAnElement(NodeId),
    /// A text operation targeted a node that is not character data.
    #[error("{0} is not character data")]
    NotCharacterData(NodeId),
}

impl DomError {
    #[must_use]
    pub fn invalid_tag_name(raw: impl Into<String>) -> Self {
        Self::InvalidTagName(raw.into())
    }

    #[must_use]
    pub fn invalid_attribute_name(raw: impl Into<String>) -> Self {
        Self::InvalidAttributeName(raw.into())
    }
}
