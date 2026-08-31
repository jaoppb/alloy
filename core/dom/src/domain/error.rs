//! [`DomError`] — the one typed error for `core/dom`.
//!
//! (`ADR-0011` item 4 applied to a domain crate). `core/dom` never names
//! `EngineError`; the `DomError` → `EngineError::Dom` mapping is the
//! `core/runtime/rhai` adapter's job at roadmap I1. `#[non_exhaustive]` so that
//! mapping keeps compiling as variants are added.

use core::fmt;

use crate::domain::node::NodeId;

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DomError {
    /// The id addresses an empty (never-used or tombstoned) arena slot.
    NodeNotFound(NodeId),
    /// The requested `append` / `insert` would make the tree cyclic.
    WouldCycle,
    /// `parent` and `child` are the same node.
    SelfParent,
    /// `detach` / `remove` was called on the `Document` root.
    CannotDetachDocument,
    /// The chosen parent is a `Text` or `Comment` node and cannot hold children.
    CannotHaveChildren(NodeId),
    /// A tag string failed [`crate::TagName`] validation.
    InvalidTagName(String),
    /// An attribute name failed [`crate::AttributeName`] validation.
    InvalidAttributeName(String),
    /// A tag / attribute operation targeted a non-`Element` node.
    NotAnElement(NodeId),
    /// A text operation targeted a node that is not character data.
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

impl fmt::Display for DomError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeNotFound(id) => write!(formatter, "{id} does not exist"),
            Self::WouldCycle => formatter.write_str("operation would make the tree cyclic"),
            Self::SelfParent => formatter.write_str("a node cannot be its own parent"),
            Self::CannotDetachDocument => {
                formatter.write_str("the document root cannot be detached or removed")
            }
            Self::CannotHaveChildren(id) => write!(formatter, "{id} cannot hold children"),
            Self::InvalidTagName(raw) => write!(formatter, "not a valid tag name: {raw:?}"),
            Self::InvalidAttributeName(raw) => {
                write!(formatter, "not a valid attribute name: {raw:?}")
            }
            Self::NotAnElement(id) => write!(formatter, "{id} is not an element"),
            Self::NotCharacterData(id) => write!(formatter, "{id} is not character data"),
        }
    }
}

impl std::error::Error for DomError {}
