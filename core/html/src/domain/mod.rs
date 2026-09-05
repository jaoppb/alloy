//! HTML domain types and core models.

pub mod error;
pub mod tag;
pub mod token;

pub use error::HtmlError;
pub use tag::{
    closes_list_item, closes_paragraph, is_block_tag, is_heading_tag, is_rawtext_tag, is_void_tag,
};
pub use token::{AttributeEntry, AttributeList, DoctypeToken, TagToken, Token};
