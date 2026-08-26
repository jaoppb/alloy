#![forbid(unsafe_code)]

//! # Core DOM (`core/dom`)
//!
//! Document Object Model tree implementation using an indexed arena pattern (`DomTree`).
//! Enforces structural invariants (acyclicity, single-parent linkage) and provides
//! safe scripting bridges to the abstract engine layer (PRD-001, PRD-002, ADR-0010).

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use domain::attribute::{AttributeMap, AttributeName, AttributeValue};
pub use domain::children::Children;
pub use domain::error::DomError;
pub use domain::node::DomNode;
pub use domain::node_data::NodeData;
pub use domain::node_id::NodeId;
pub use domain::service::DomService;
pub use domain::slot::Slot;
pub use domain::tag_name::TagName;
pub use domain::tree::{DomTree, QuirksMode};
pub use infrastructure::bridge::register_dom_bindings;
