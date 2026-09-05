//! Infrastructure adapters and implementations for HTML tokenization and tree construction.

#[cfg(feature = "dom")]
pub mod dom_sink;
pub mod mock;
pub mod tokenizer;
pub mod tree_builder;

#[cfg(feature = "dom")]
pub use dom_sink::DomTreeSink;
pub use mock::{MockEvent, MockTreeSink};
pub use tokenizer::{Tokenizer, TokenizerRunResult};
pub use tree_builder::TreeBuilder;
