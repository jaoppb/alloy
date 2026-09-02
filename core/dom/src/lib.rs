//! # `dom` — the DOM tree aggregate
//!
//! The **Skeleton-side** structural domain for the document tree (`ADR-0003`):
//! an arena [`DomTree`] that owns every node and enforces the five invariants of
//! the v0.2 report §2.2 (acyclicity, single parent, no self-parent, an
//! irremovable `Document` root, `Children` ⇄ `parent` coherence). Mutation only
//! ever happens through [`DomTree`]'s methods (Object Calisthenics rule 8).
//!
//! This crate has **zero dependencies** and names no engine type (v0.2 report
//! decision 2.1). Making a node scriptable — the `NodeHandle` bridge and the
//! `DomError` → `EngineError::Dom` mapping — is `core/runtime/rhai`'s job at
//! roadmap point I1, not this crate's.
//!
//! ## Layout (`ADR-0010` §1)
//!
//! - [`domain`] — [`DomTree`], [`NodeId`], [`NodeKind`] / [`ElementData`], the
//!   value objects ([`TagName`], [`AttributeName`], [`AttributeValue`],
//!   [`TextContent`], [`CommentContent`]), the first-class collections
//!   ([`Children`], [`AttributeMap`]), the typed [`DomError`], and the
//!   [`Descendants`] / [`Ancestors`] iterators.
//! - [`application`] — [`serialize_html`], a pure deterministic serializer.

#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

pub mod application;
pub mod domain;

pub use application::serialize::serialize_html;
pub use domain::{
    attributes::{AttributeMap, AttributeName, AttributeValue},
    entity::HtmlEntity,
    error::DomError,
    node::{ElementData, NodeId, NodeKind},
    tag_name::TagName,
    text::{CommentContent, TextContent},
    traversal::{Ancestors, Children, Descendants},
    tree::DomTree,
};
