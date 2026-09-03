//! Innermost layer (`ADR-0010` §1).
//!
//! The [`tree::DomTree`] aggregate, its value objects and first-class
//! collections, its one typed [`error::DomError`], and the non-recursive
//! [`traversal`] iterators. Zero I/O, zero dependencies; the nine Object
//! Calisthenics rules apply in full — `core/dom` takes no exception.

pub mod attributes;
pub mod entity;
pub mod error;
pub mod node;
pub mod tag_name;
pub mod text;
pub mod traversal;
pub mod tree;
