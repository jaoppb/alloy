//! # `dom` — the DOM tree aggregate
//!
//! The **Skeleton-side** structural domain for the document tree (`ADR-0003`):
//! an arena [`DomTree`] that owns every node and enforces the five invariants of
//! the v0.2 report §2.2 (acyclicity, single parent, no self-parent, an
//! irremovable `Document` root, `Children` ⇄ `parent` coherence). Mutation only
//! ever happens through [`DomTree`]'s methods (Object Calisthenics rule 8).
//!
//! This crate depends only on the `html` vocabulary crate (`ADR-0024`; it
//! supersedes v0.2 report decision 2.1's "zero dependencies") and names no
//! engine type. Making a node scriptable — the `NodeHandle` bridge and the
//! `DomError` → `EngineError::Subsystem { subsystem: SubsystemName::Dom, .. }`
//! mapping — is `core/runtime/rhai`'s job at roadmap point I1, not this
//! crate's.
//!
//! ## Layout (`ADR-0010` §1)
//!
//! - [`domain`] — [`DomTree`], [`NodeId`], [`NodeKind`] / [`ElementData`], the
//!   value objects ([`TextContent`], [`CommentContent`]; the tag and attribute
//!   vocabulary is `html`'s), the first-class collections
//!   ([`Children`], [`AttributeMap`]), the typed [`DomError`], and the
//!   [`Descendants`] / [`Ancestors`] iterators.
//! - [`application`] — [`serialize_html`], a pure deterministic serializer.
//! - [`infrastructure`] — [`DomTreeSink`], the default `html::TreeSink` adapter, and [`parse`].

#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::serialize::serialize_html;
pub use domain::{
    attributes::AttributeMap,
    error::DomError,
    node::{ElementData, NodeId, NodeKind},
    text::{CommentContent, TextContent},
    traversal::{Ancestors, Children, Descendants},
    tree::DomTree,
};
pub use infrastructure::html_sink::{DomTreeSink, ParseOutcome, parse};
