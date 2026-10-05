//! HTML domain types and core models.

pub mod attribute;
pub mod diagnostic;
pub mod error;
pub mod handle;
pub mod location;
pub mod tag;
pub mod text;
pub mod token;

pub use attribute::{
    AttributeEntry, AttributeList, AttributeName, AttributeValue, DuplicateAttribute,
};
pub use diagnostic::{Diagnostics, ParseDiagnostic, ParseErrorCode};
pub use error::HtmlError;
pub use handle::NodeHandle;
pub use location::SourceLocation;
pub use tag::{
    TagName, closes_list_item, closes_paragraph, is_block_tag, is_heading_tag, is_implied_end_tag,
    is_rawtext_tag, is_void_tag,
};
pub use text::Text;
pub use token::{DoctypeToken, TagToken, Token};
