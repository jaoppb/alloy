//! HTML domain types and core models.

pub mod attribute;
pub mod error;
pub mod handle;
pub mod location;
pub mod tag;
pub mod tag_name;
pub mod text;
pub mod token;

pub use attribute::{AttributeEntry, AttributeList, AttributeName, AttributeValue};
pub use error::HtmlError;
pub use handle::NodeHandle;
pub use location::SourceLocation;
pub use tag::{
    closes_list_item, closes_paragraph, is_block_tag, is_heading_tag, is_rawtext_tag, is_void_tag,
};
pub use tag_name::TagName;
pub use text::Text;
pub use token::{DoctypeToken, TagToken, Token};
