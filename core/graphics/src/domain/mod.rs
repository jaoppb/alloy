//! Zero-I/O value objects: units, geometry, colour, resource handles, the
//! backend tier, and the one typed error of this port (`ADR-0010` §1).

pub mod color;
pub mod command;
pub mod command_index;
pub mod command_kind;
pub(crate) mod convert;
pub mod display_list;
pub mod error;
pub mod font;
pub mod geometry;
pub mod image;
pub mod path;
pub mod tier;
pub mod unit;
