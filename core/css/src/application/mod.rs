//! The three replaceable ports, the explicit `dom::DomTree → DomSnapshot`
//! mapping, selector matching, the document's own stylesheet collection, and
//! the port conformance suite (`ADR-0010` §1).

pub mod collect_sheets;
pub mod conformance;
pub mod matching;
pub mod ports;
pub mod snapshot;

pub use collect_sheets::collect_style_sheets;
pub use matching::{matches, strongest_match};
pub use ports::{CascadeResolver, LayoutEngine, TextMeasurer};
pub use snapshot::snapshot;
