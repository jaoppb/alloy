//! The three replaceable ports, the explicit `dom::DomTree → DomSnapshot`
//! mapping, and the port conformance suite (`ADR-0010` §1).

pub mod conformance;
pub mod ports;
pub mod snapshot;

pub use ports::{CascadeResolver, LayoutEngine, TextMeasurer};
pub use snapshot::snapshot;
