//! The browsing context and the ports it runs on (`ADR-0010`).

mod browsing_context;
mod error_page;
mod stylesheet;
mod subresource;

pub use browsing_context::BrowsingContext;
pub use error_page::{ErrorPageError, error_page};
pub use stylesheet::{StylesheetRejected, parse_author_stylesheet};
pub use subresource::{SubresourceDiscoverer, document_base_url};
