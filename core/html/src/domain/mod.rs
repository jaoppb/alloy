//! HTML domain types and core models.

pub mod attribute;
pub mod diagnostic;
pub mod error;
pub mod handle;
pub mod location;
pub mod named_reference;
mod named_references;
pub mod tag;
pub mod text;
pub mod token;

pub use attribute::{
    AttributeEntry, AttributeList, AttributeName, AttributeValue, DuplicateAttribute,
};
pub use diagnostic::{Diagnostics, ParseDiagnostic, ParseErrorCode};
pub use error::{HtmlError, InvalidAttributeName, InvalidTagName};
pub use handle::NodeHandle;
pub use location::SourceLocation;
pub use named_reference::{NamedCharacterReference, ReferenceMatch};
pub use tag::TagName;
pub use text::Text;
pub use token::{DoctypeToken, TagToken, Token};
