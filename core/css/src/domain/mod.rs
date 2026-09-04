//! Zero-I/O value objects, computed-value enums, the four boundary aggregates
//! and the one typed error of this port (`ADR-0010` §1).

pub mod color;
pub mod computed;
pub mod declaration;
pub mod dom_snapshot;
pub mod error;
pub mod identifier;
pub mod layout_box_tree;
pub mod length;
pub mod media;
pub mod parse_notes;
pub mod selector;
pub mod specificity;
pub mod styled_tree;
pub mod stylesheet_set;
pub mod text;
pub mod viewport;
