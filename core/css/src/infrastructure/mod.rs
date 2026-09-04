//! The placeholder Rust adapters for the CSS ports and the port mocks
//! (`ADR-0010` §1).
//!
//! `UaCascade`, `BlockLayout` and `MonospaceMetrics` are deliberately minimal —
//! B2 replaces the cascade, B4 replaces the layout engine. They exist so the
//! contract is dogfooded from B0 (`PRD-007` §3.5). The mocks prove the ports
//! swap (`PRD-007:94`).

pub mod cascade;
pub mod layout;
pub mod mock;
pub mod parser;
pub mod text_metrics;
pub mod ua_sheet;

pub use cascade::UaCascade;
pub use layout::BlockLayout;
pub use mock::{MockCascadeResolver, MockLayoutEngine, MockTextMeasurer};
pub use text_metrics::MonospaceMetrics;
